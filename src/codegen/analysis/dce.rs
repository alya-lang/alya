use crate::ast::*;
use std::collections::{HashMap, HashSet};

/// Eliminates unreferenced functions and unused structs from the AST.
/// Starts from entry points (top-level statements and any `main` function)
/// and transitively retains only reachable functions and structs.
pub fn eliminate_dead_code(program: &Program) -> Program {
    let mut function_defs: HashMap<String, Stmt> = HashMap::new();
    let mut free_functions_by_bare: HashMap<String, Vec<String>> = HashMap::new();
    let mut struct_methods: HashMap<String, Vec<String>> = HashMap::new();
    let mut methods_by_name: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut struct_constructors: HashMap<String, Vec<String>> = HashMap::new();

    let mut struct_defs: HashMap<String, Stmt> = HashMap::new();
    let mut structs_by_bare: HashMap<String, Vec<String>> = HashMap::new();
    let mut known_struct_names: HashSet<String> = HashSet::new();

    struct InterfaceDefEntry {
        methods: Vec<String>,
        embedded: Vec<String>,
        type_refs: Vec<String>,
        is_pub: bool,
    }

    let mut interface_defs: HashMap<String, InterfaceDefEntry> = HashMap::new();
    let mut interfaces_by_bare: HashMap<String, Vec<String>> = HashMap::new();

    for stmt in &program.statements {
        match stmt.inner_stmt() {
            Stmt::StructDef { name, .. } => {
                let bare = bare_name(name);
                struct_defs.insert(name.clone(), stmt.clone());
                structs_by_bare
                    .entry(bare.to_string())
                    .or_default()
                    .push(name.clone());
                known_struct_names.insert(name.clone());
                known_struct_names.insert(bare.to_string());
            }
            Stmt::InterfaceDef {
                name,
                methods,
                embedded,
            } => {
                let bare = bare_name(name);
                let mut type_refs = Vec::new();
                for m in methods {
                    for pt in m.param_types.iter().flatten() {
                        let mut set = HashSet::new();
                        collect_type_names(pt, &mut set);
                        type_refs.extend(set);
                    }
                    if let Some(rt) = &m.return_type {
                        let mut set = HashSet::new();
                        collect_type_names(rt, &mut set);
                        type_refs.extend(set);
                    }
                }
                for emb in embedded {
                    type_refs.push(emb.clone());
                    type_refs.push(bare_name(emb).to_string());
                }
                let entry = InterfaceDefEntry {
                    methods: methods.iter().map(|m| m.name.clone()).collect(),
                    embedded: embedded.clone(),
                    type_refs,
                    is_pub: stmt.is_pub(),
                };
                interface_defs.insert(name.clone(), entry);
                interfaces_by_bare
                    .entry(bare.to_string())
                    .or_default()
                    .push(name.clone());
            }
            _ => {}
        }
    }

    let mut top_level: Vec<&Stmt> = Vec::new();

    for stmt in &program.statements {
        match stmt.inner_stmt() {
            Stmt::Function { name, body, .. } => {
                let bare = bare_name(name);
                function_defs.insert(name.clone(), stmt.clone());

                let mut inits = HashSet::new();
                for s in body {
                    collect_struct_inits_in_stmt(s, &mut inits);
                }
                for init_st in inits {
                    struct_constructors
                        .entry(init_st)
                        .or_default()
                        .push(name.clone());
                }

                let bare_fn = name.rsplit("::").next().unwrap_or(name);
                let base_fn = bare_fn.split("__spk__").next().unwrap_or(bare_fn);
                let parts: Vec<&str> = base_fn.split("__").collect();
                let is_struct_method = if parts.len() >= 2 && !parts[0].is_empty() {
                    let st_prefix = parts[0];
                    let full_st = name
                        .rsplit_once("::")
                        .map(|(m, _)| format!("{}::{}", m, st_prefix));
                    known_struct_names.contains(st_prefix)
                        || full_st
                            .as_ref()
                            .is_some_and(|s| known_struct_names.contains(s))
                } else {
                    false
                };

                if is_struct_method {
                    let st_prefix = parts[0];
                    let method_name = parts[1];
                    struct_methods
                        .entry(st_prefix.to_string())
                        .or_default()
                        .push(name.clone());
                    if let Some(mod_prefix) = name.rsplit_once("::").map(|(m, _)| m) {
                        let full_st = format!("{}::{}", mod_prefix, st_prefix);
                        struct_methods
                            .entry(full_st)
                            .or_default()
                            .push(name.clone());
                    }
                    methods_by_name
                        .entry(method_name.to_string())
                        .or_default()
                        .push((st_prefix.to_string(), name.clone()));
                } else {
                    free_functions_by_bare
                        .entry(bare.to_string())
                        .or_default()
                        .push(name.clone());
                }
            }
            Stmt::StructDef { .. } | Stmt::InterfaceDef { .. } => {}
            _ => {
                top_level.push(stmt);
            }
        }
    }

    // Check if there is any `main` function defined
    let has_main = function_defs.contains_key("main")
        || free_functions_by_bare.contains_key("main")
        || function_defs.keys().any(|f| bare_name(f) == "main");

    // If there are no top-level statements and no `main` function, this is a pure
    // library file compiled in isolation. Keep all definitions to preserve
    // compilation, except unreferenced suite entries (`test_*` / `__test_*` /
    // `@test` / `@bench` / `__bench_*`): the normal build path never
    // synthesizes suite calls, so those functions would be dead weight in the
    // artifact. The `test` / `bench` pipeline keeps entries alive through
    // synthesized top-level calls instead.
    if top_level.is_empty() && !has_main {
        let mut referenced: HashSet<String> = HashSet::new();
        for stmt in function_defs.values() {
            if let Stmt::Function { body, defaults, .. } = stmt.inner_stmt() {
                for d in defaults.iter().flatten() {
                    collect_references_in_expr(d, &mut referenced);
                }
                for s in body {
                    collect_references_in_stmt(s, &mut referenced);
                }
            }
        }
        let referenced_bare: HashSet<String> = referenced
            .iter()
            .map(|r| bare_name(r).to_string())
            .collect();
        let mut pruned_statements = Vec::new();
        for stmt in &program.statements {
            match stmt.inner_stmt() {
                Stmt::Function {
                    name, attributes, ..
                } if is_suite_entry(name, attributes) => {
                    if referenced.contains(name) || referenced_bare.contains(bare_name(name)) {
                        pruned_statements.push(stmt.clone());
                    }
                }
                _ => pruned_statements.push(stmt.clone()),
            }
        }
        return Program {
            statements: pruned_statements,
        };
    }

    let mut reachable_functions: HashSet<String> = HashSet::new();
    let mut reachable_structs: HashSet<String> = HashSet::new();
    let mut reachable_interfaces: HashSet<String> = HashSet::new();
    let mut called_methods: HashSet<String> = HashSet::new();

    enum WorkItem {
        Function(String),
        Struct(String),
        Interface(String),
    }

    let mut worklist: Vec<WorkItem> = Vec::new();

    let mark_function =
        |fn_name: &str, reachable_functions: &mut HashSet<String>, worklist: &mut Vec<WorkItem>| {
            if reachable_functions.insert(fn_name.to_string()) {
                worklist.push(WorkItem::Function(fn_name.to_string()));
            }
        };

    let activate_struct_methods = |st_name: &str,
                                   struct_methods: &HashMap<String, Vec<String>>,
                                   called_methods: &HashSet<String>,
                                   function_defs: &HashMap<String, Stmt>,
                                   has_main: bool,
                                   reachable_functions: &mut HashSet<String>,
                                   worklist: &mut Vec<WorkItem>| {
        let bare_st = bare_name(st_name);
        let mut check_prefix = |prefix: &str| {
            if let Some(fns) = struct_methods.get(prefix) {
                for fn_name in fns {
                    let mb = bare_name(fn_name);
                    let is_called = called_methods.contains(mb)
                        || called_methods.contains(fn_name)
                        || mb == "to_string"
                        || mb.starts_with("operator");
                    let is_exported =
                        function_defs
                            .get(fn_name)
                            .is_some_and(|s| match s.inner_stmt() {
                                Stmt::Function { attributes, .. } => {
                                    attributes.iter().any(|a| a.name == "export")
                                }
                                _ => false,
                            });
                    let is_pub_in_lib =
                        !has_main && function_defs.get(fn_name).is_some_and(|s| s.is_pub());

                    if (is_called || is_exported || is_pub_in_lib)
                        && reachable_functions.insert(fn_name.clone())
                    {
                        worklist.push(WorkItem::Function(fn_name.clone()));
                    }
                }
            }
        };
        check_prefix(st_name);
        if bare_st != st_name {
            check_prefix(bare_st);
        }
    };

    let resolve_symbol = |symbol: &str,
                          function_defs: &HashMap<String, Stmt>,
                          free_functions_by_bare: &HashMap<String, Vec<String>>,
                          struct_defs: &HashMap<String, Stmt>,
                          structs_by_bare: &HashMap<String, Vec<String>>,
                          interface_defs: &HashMap<String, InterfaceDefEntry>,
                          interfaces_by_bare: &HashMap<String, Vec<String>>,
                          struct_methods: &HashMap<String, Vec<String>>,
                          called_methods: &HashSet<String>,
                          has_main: bool,
                          reachable_functions: &mut HashSet<String>,
                          reachable_structs: &mut HashSet<String>,
                          reachable_interfaces: &mut HashSet<String>,
                          worklist: &mut Vec<WorkItem>| {
        let bare = bare_name(symbol);

        // Check exact function
        if function_defs.contains_key(symbol) && reachable_functions.insert(symbol.to_string()) {
            worklist.push(WorkItem::Function(symbol.to_string()));
        }
        let colon_symbol = symbol.replace("__", "::");
        if colon_symbol != symbol
            && function_defs.contains_key(&colon_symbol)
            && reachable_functions.insert(colon_symbol.clone())
        {
            worklist.push(WorkItem::Function(colon_symbol));
        }
        let mangled_symbol = symbol.replace("::", "__");
        if mangled_symbol != symbol
            && function_defs.contains_key(&mangled_symbol)
            && reachable_functions.insert(mangled_symbol.clone())
        {
            worklist.push(WorkItem::Function(mangled_symbol));
        }

        // Also check any candidate free functions matching bare name
        let is_st_method = if let Some((st_prefix, _)) = symbol.split_once("__") {
            known_struct_names.contains(st_prefix)
                || struct_defs.contains_key(st_prefix)
                || structs_by_bare.contains_key(st_prefix)
        } else {
            false
        };
        if !is_st_method {
            if let Some(candidates) = free_functions_by_bare.get(bare) {
                for cand in candidates {
                    if reachable_functions.insert(cand.clone()) {
                        worklist.push(WorkItem::Function(cand.clone()));
                    }
                }
            }
        }

        // Check exact struct
        if struct_defs.contains_key(symbol) && reachable_structs.insert(symbol.to_string()) {
            activate_struct_methods(
                symbol,
                struct_methods,
                called_methods,
                function_defs,
                has_main,
                reachable_functions,
                worklist,
            );
            worklist.push(WorkItem::Struct(symbol.to_string()));
        }
        if !symbol.contains("__") {
            if let Some(candidates) = structs_by_bare.get(bare) {
                for cand in candidates {
                    if reachable_structs.insert(cand.clone()) {
                        activate_struct_methods(
                            cand,
                            struct_methods,
                            called_methods,
                            function_defs,
                            has_main,
                            reachable_functions,
                            worklist,
                        );
                        worklist.push(WorkItem::Struct(cand.clone()));
                    }
                }
            }
        }

        // Check exact interface
        if interface_defs.contains_key(symbol) && reachable_interfaces.insert(symbol.to_string()) {
            worklist.push(WorkItem::Interface(symbol.to_string()));
        }
        if !symbol.contains("__") {
            if let Some(candidates) = interfaces_by_bare.get(bare) {
                for cand in candidates {
                    if reachable_interfaces.insert(cand.clone()) {
                        worklist.push(WorkItem::Interface(cand.clone()));
                    }
                }
            }
        }
    };

    // 1. If any `main` function exists, it is always a root
    if let Some(main_fns) = free_functions_by_bare.get("main") {
        for m in main_fns {
            mark_function(m, &mut reachable_functions, &mut worklist);
        }
    } else if let Some(stmt) = function_defs.get("main") {
        if matches!(stmt.inner_stmt(), Stmt::Function { .. }) {
            mark_function("main", &mut reachable_functions, &mut worklist);
        }
    } else {
        // In library or module context without main(), only pub items are roots.
        for (name, stmt) in &function_defs {
            if stmt.is_pub() {
                mark_function(name, &mut reachable_functions, &mut worklist);
            }
        }
        for (name, stmt) in &struct_defs {
            if stmt.is_pub() && reachable_structs.insert(name.clone()) {
                activate_struct_methods(
                    name,
                    &struct_methods,
                    &called_methods,
                    &function_defs,
                    has_main,
                    &mut reachable_functions,
                    &mut worklist,
                );
                worklist.push(WorkItem::Struct(name.clone()));
            }
        }
        for (name, entry) in &interface_defs {
            if entry.is_pub && reachable_interfaces.insert(name.clone()) {
                worklist.push(WorkItem::Interface(name.clone()));
            }
        }
    }

    // 1b. `@export`ed functions are library entry points for native consumers
    for (name, stmt) in &function_defs {
        let exported = match stmt.inner_stmt() {
            Stmt::Function { attributes, .. } => attributes.iter().any(|a| a.name == "export"),
            _ => false,
        };
        if exported {
            mark_function(name, &mut reachable_functions, &mut worklist);
        }
    }

    let mut all_reachable_refs: HashSet<String> = HashSet::new();

    // 2. All top-level statements are roots
    let mut initial_refs = HashSet::new();
    let mut initial_calls = HashSet::new();
    let empty_locals = HashSet::new();
    for stmt in &top_level {
        collect_references_in_stmt_scoped(
            stmt,
            &empty_locals,
            &mut initial_refs,
            &mut initial_calls,
        );
    }
    all_reachable_refs.extend(initial_refs.iter().cloned());

    for call_name in initial_calls {
        let mb = bare_name(&call_name);
        called_methods.insert(mb.to_string());
        called_methods.insert(call_name.clone());
        if let Some(cands) = methods_by_name.get(mb) {
            for (_st_prefix, fn_name) in cands {
                if reachable_functions.insert(fn_name.clone()) {
                    worklist.push(WorkItem::Function(fn_name.clone()));
                }
            }
        }
    }

    for r in &initial_refs {
        resolve_symbol(
            r,
            &function_defs,
            &free_functions_by_bare,
            &struct_defs,
            &structs_by_bare,
            &interface_defs,
            &interfaces_by_bare,
            &struct_methods,
            &called_methods,
            has_main,
            &mut reachable_functions,
            &mut reachable_structs,
            &mut reachable_interfaces,
            &mut worklist,
        );
    }

    // 3. Process worklist until fixpoint
    while let Some(item) = worklist.pop() {
        let mut item_refs = HashSet::new();
        let mut new_calls = HashSet::new();
        match item {
            WorkItem::Function(fn_name) => {
                if let Some(stmt) = function_defs.get(&fn_name) {
                    if let Stmt::Function {
                        params,
                        param_types,
                        return_type,
                        defaults,
                        body,
                        ..
                    } = stmt.inner_stmt()
                    {
                        // If this function is a struct method (e.g. "ev::TcpServer__on_error"),
                        // keep the receiver struct reachable!
                        let bare_fn = fn_name.rsplit("::").next().unwrap_or(&fn_name);
                        let base_fn = bare_fn.split("__spk__").next().unwrap_or(bare_fn);
                        let parts: Vec<&str> = base_fn.split("__").collect();
                        let is_st = parts.len() >= 2
                            && !parts[0].is_empty()
                            && (known_struct_names.contains(parts[0])
                                || fn_name
                                    .rsplit_once("::")
                                    .map(|(m, _)| format!("{}::{}", m, parts[0]))
                                    .is_some_and(|s| known_struct_names.contains(&s)));
                        if is_st {
                            item_refs.insert(parts[0].to_string());
                        }

                        for t in param_types.iter().flatten() {
                            collect_type_names(t, &mut item_refs);
                        }
                        if let Some(rt) = return_type {
                            collect_type_names(rt, &mut item_refs);
                        }

                        let mut locals = HashSet::new();
                        for p in params {
                            locals.insert(p.clone());
                        }
                        collect_local_vars(body, &mut locals);

                        for expr in defaults.iter().flatten() {
                            collect_references_in_expr_scoped(
                                expr,
                                &locals,
                                &mut item_refs,
                                &mut new_calls,
                            );
                        }
                        for s in body {
                            collect_references_in_stmt_scoped(
                                s,
                                &locals,
                                &mut item_refs,
                                &mut new_calls,
                            );
                        }
                    }
                }
            }
            WorkItem::Struct(st_name) => {
                let bare_st = bare_name(&st_name);
                if let Some(cfns) = struct_constructors.get(&st_name) {
                    for cfn in cfns {
                        if reachable_functions.insert(cfn.clone()) {
                            worklist.push(WorkItem::Function(cfn.clone()));
                        }
                    }
                }
                if bare_st != st_name {
                    if let Some(cfns) = struct_constructors.get(bare_st) {
                        for cfn in cfns {
                            if reachable_functions.insert(cfn.clone()) {
                                worklist.push(WorkItem::Function(cfn.clone()));
                            }
                        }
                    }
                }

                if let Some(stmt) = struct_defs.get(&st_name) {
                    if let Stmt::StructDef {
                        field_types,
                        defaults,
                        ..
                    } = stmt.inner_stmt()
                    {
                        for t in field_types.iter().flatten() {
                            collect_type_names(t, &mut item_refs);
                        }
                        let empty_locals = HashSet::new();
                        for expr in defaults.iter().flatten() {
                            collect_references_in_expr_scoped(
                                expr,
                                &empty_locals,
                                &mut item_refs,
                                &mut new_calls,
                            );
                        }
                    }
                }
            }
            WorkItem::Interface(iface_name) => {
                let bare_iface = bare_name(&iface_name);
                let entry = interface_defs
                    .get(&iface_name)
                    .or_else(|| interface_defs.get(bare_iface));
                if let Some(entry) = entry {
                    for t in &entry.type_refs {
                        item_refs.insert(t.clone());
                    }
                    for emb in &entry.embedded {
                        item_refs.insert(emb.clone());
                        item_refs.insert(bare_name(emb).to_string());
                    }
                    for m_name in &entry.methods {
                        let mb = bare_name(m_name);
                        called_methods.insert(mb.to_string());
                        called_methods.insert(m_name.clone());
                        if let Some(cands) = methods_by_name.get(mb) {
                            for (st_prefix, fn_name) in cands {
                                let bare_st = bare_name(st_prefix);
                                let is_st_reachable = reachable_structs.contains(st_prefix)
                                    || reachable_structs.contains(bare_st)
                                    || reachable_structs.iter().any(|s| bare_name(s) == bare_st);
                                if is_st_reachable && reachable_functions.insert(fn_name.clone()) {
                                    worklist.push(WorkItem::Function(fn_name.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process newly discovered called methods: activate candidate methods
        for call_name in new_calls {
            let mb = bare_name(&call_name);
            let inserted_mb = called_methods.insert(mb.to_string());
            let inserted_raw = called_methods.insert(call_name.clone());
            if inserted_mb || inserted_raw {
                if let Some(cands) = methods_by_name.get(mb) {
                    for (_st_prefix, fn_name) in cands {
                        if reachable_functions.insert(fn_name.clone()) {
                            worklist.push(WorkItem::Function(fn_name.clone()));
                        }
                    }
                }
            }
        }

        all_reachable_refs.extend(item_refs.iter().cloned());
        for r in &item_refs {
            resolve_symbol(
                r,
                &function_defs,
                &free_functions_by_bare,
                &struct_defs,
                &structs_by_bare,
                &interface_defs,
                &interfaces_by_bare,
                &struct_methods,
                &called_methods,
                has_main,
                &mut reachable_functions,
                &mut reachable_structs,
                &mut reachable_interfaces,
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
            Stmt::InterfaceDef { name, .. } => {
                let bare = bare_name(name);
                if reachable_interfaces.contains(name)
                    || reachable_interfaces.contains(bare)
                    || reachable_interfaces.iter().any(|i| bare_name(i) == bare)
                {
                    pruned_statements.push(stmt.clone());
                }
            }
            Stmt::ExternBlock {
                abi,
                lib,
                functions,
            } => {
                // In library context without main(), preserve all pub extern blocks
                if !has_main && stmt.is_pub() {
                    pruned_statements.push(stmt.clone());
                    continue;
                }

                let reachable_extern_fns: Vec<ExternFnDecl> = functions
                    .iter()
                    .filter(|f| {
                        let bare = bare_name(&f.name);
                        all_reachable_refs.contains(&f.name)
                            || all_reachable_refs.contains(bare)
                            || all_reachable_refs.iter().any(|r| bare_name(r) == bare)
                    })
                    .cloned()
                    .collect();

                if !reachable_extern_fns.is_empty() {
                    let filtered = Stmt::ExternBlock {
                        abi: abi.clone(),
                        lib: lib.clone(),
                        functions: reachable_extern_fns,
                    };
                    if stmt.is_pub() {
                        pruned_statements.push(Stmt::Pub(Box::new(filtered)));
                    } else {
                        pruned_statements.push(filtered);
                    }
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

pub(crate) fn bare_name(name: &str) -> &str {
    let bare = name.rsplit("::").next().unwrap_or(name);
    bare.rsplit("__").next().unwrap_or(bare)
}

/// Returns true for test/bench suite entries: lowered `test` / `bench`
/// blocks (`__test_*` / `__bench_*`), `@test` / `@bench`-attributed
/// functions, and `test_*`-named functions. Mirrors the discovery rule in
/// `tools::test_runner::discover_suite_entry_points`, plus the `test_*`
/// naming convention.
pub(crate) fn is_suite_entry(name: &str, attributes: &[Attribute]) -> bool {
    let segment = name.rsplit("::").next().unwrap_or(name);
    if segment.starts_with("__test_") || segment.starts_with("__bench_") {
        return true;
    }
    if attributes
        .iter()
        .any(|a| a.name == "test" || a.name == "bench")
        && !name.contains("__")
    {
        return true;
    }
    // Note: the bare name of a lowered `__test_*` block is `test_*`, so this
    // also covers lowered test blocks under qualified names.
    bare_name(name).starts_with("test_")
}

pub(crate) fn is_primitive_type(t: &str) -> bool {
    matches!(
        t,
        "int"
            | "float"
            | "bool"
            | "string"
            | "str"
            | "void"
            | "any"
            | "ptr"
            | "byte"
            | "char"
            | "f64"
            | "f32"
            | "i64"
            | "i32"
            | "i16"
            | "i8"
            | "u64"
            | "u32"
            | "u16"
            | "u8"
            | "array"
            | "map"
            | "nil"
            | "null"
            | "none"
    )
}

pub(crate) fn collect_type_names(t: &str, refs: &mut HashSet<String>) {
    let mut word = String::new();
    for c in t.chars() {
        if c.is_alphanumeric() || c == '_' || c == ':' {
            word.push(c);
        } else {
            if !word.is_empty() {
                let bare = bare_name(&word);
                if !is_primitive_type(bare) && bare != "Self" && bare != "fn" && bare != "weak" {
                    refs.insert(word.clone());
                    refs.insert(bare.to_string());
                }
                word.clear();
            }
        }
    }
    if !word.is_empty() {
        let bare = bare_name(&word);
        if !is_primitive_type(bare) && bare != "Self" && bare != "fn" && bare != "weak" {
            refs.insert(word.clone());
            refs.insert(bare.to_string());
        }
    }
}

pub(crate) fn collect_local_vars(stmts: &[Stmt], vars: &mut HashSet<String>) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                vars.insert(name.clone());
            }
            Stmt::For { var, body, .. } => {
                vars.insert(var.clone());
                collect_local_vars(body, vars);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                vars.insert(var.clone());
                if let Some(ref v) = value_var {
                    vars.insert(v.clone());
                }
                collect_local_vars(body, vars);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_local_vars(then_block, vars);
                if let Some(ref eb) = else_block {
                    collect_local_vars(eb, vars);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                collect_local_vars(body, vars);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                catch_var,
                finally_block,
                ..
            } => {
                collect_local_vars(try_block, vars);
                if let Some(ref cv) = catch_var {
                    vars.insert(cv.clone());
                }
                collect_local_vars(catch_block, vars);
                if let Some(ref fb) = finally_block {
                    collect_local_vars(fb, vars);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn collect_references_in_stmt(stmt: &Stmt, refs: &mut HashSet<String>) {
    let empty_locals = HashSet::new();
    let mut called_methods = HashSet::new();
    collect_references_in_stmt_scoped(stmt, &empty_locals, refs, &mut called_methods);
}

pub(crate) fn collect_references_in_stmt_scoped(
    stmt: &Stmt,
    locals: &HashSet<String>,
    refs: &mut HashSet<String>,
    called_methods: &mut HashSet<String>,
) {
    match stmt.inner_stmt() {
        Stmt::Expr(expr) | Stmt::Say(expr) => {
            collect_references_in_expr_scoped(expr, locals, refs, called_methods);
        }
        Stmt::Let {
            value, type_ann, ..
        } => {
            if let Some(t) = type_ann {
                collect_type_names(t, refs);
            }
            collect_references_in_expr_scoped(value, locals, refs, called_methods);
        }
        Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            collect_references_in_expr_scoped(value, locals, refs, called_methods);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_references_in_expr_scoped(array, locals, refs, called_methods);
            collect_references_in_expr_scoped(index, locals, refs, called_methods);
            collect_references_in_expr_scoped(value, locals, refs, called_methods);
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_references_in_expr_scoped(object, locals, refs, called_methods);
            collect_references_in_expr_scoped(value, locals, refs, called_methods);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_references_in_expr_scoped(condition, locals, refs, called_methods);
            for s in then_block {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    collect_references_in_stmt_scoped(s, locals, refs, called_methods);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_references_in_expr_scoped(condition, locals, refs, called_methods);
            for s in body {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            collect_references_in_expr_scoped(start, locals, refs, called_methods);
            collect_references_in_expr_scoped(end, locals, refs, called_methods);
            for s in body {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            refs.insert("channel_recv".to_string());
            refs.insert("Channel__recv".to_string());
            collect_references_in_expr_scoped(iterable, locals, refs, called_methods);
            for s in body {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
        }
        Stmt::Return(opt_expr) | Stmt::Throw(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_references_in_expr_scoped(expr, locals, refs, called_methods);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
            for s in catch_block {
                collect_references_in_stmt_scoped(s, locals, refs, called_methods);
            }
            if let Some(finally_stmts) = finally_block {
                for s in finally_stmts {
                    collect_references_in_stmt_scoped(s, locals, refs, called_methods);
                }
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            collect_references_in_stmt_scoped(inner, locals, refs, called_methods);
        }
        Stmt::Function {
            params,
            body,
            defaults,
            ..
        } => {
            let mut fn_locals = locals.clone();
            for p in params {
                fn_locals.insert(p.clone());
            }
            collect_local_vars(body, &mut fn_locals);

            for d in defaults.iter().flatten() {
                collect_references_in_expr_scoped(d, &fn_locals, refs, called_methods);
            }
            for s in body {
                collect_references_in_stmt_scoped(s, &fn_locals, refs, called_methods);
            }
        }
        Stmt::StructDef { defaults, .. } => {
            for d in defaults.iter().flatten() {
                collect_references_in_expr_scoped(d, locals, refs, called_methods);
            }
        }
        _ => {}
    }
}

fn collect_references_in_expr(expr: &Expr, refs: &mut HashSet<String>) {
    let empty_locals = HashSet::new();
    let mut called_methods = HashSet::new();
    collect_references_in_expr_scoped(expr, &empty_locals, refs, &mut called_methods);
}

fn collect_references_in_expr_scoped(
    expr: &Expr,
    locals: &HashSet<String>,
    refs: &mut HashSet<String>,
    called_methods: &mut HashSet<String>,
) {
    match expr {
        Expr::Identifier(id) => {
            if !locals.contains(id) {
                refs.insert(id.clone());
            }
        }
        Expr::Call { name, args } => {
            refs.insert(name.clone());
            called_methods.insert(bare_name(name).to_string());
            called_methods.insert(name.clone());
            let colon_name = name.replace("__", "::");
            if colon_name != *name {
                refs.insert(colon_name);
            }
            let mangled_name = name.replace("::", "__");
            if mangled_name != *name {
                refs.insert(mangled_name);
            }
            if let Some(Expr::Identifier(recv)) = args.first() {
                refs.insert(format!("{}::{}", recv, name));
                refs.insert(format!("{}__{}", recv, name));
            }
            for arg in args {
                collect_references_in_expr_scoped(arg, locals, refs, called_methods);
            }
        }
        Expr::OptionalCall { callee, args } => {
            refs.insert(callee.clone());
            called_methods.insert(bare_name(callee).to_string());
            called_methods.insert(callee.clone());
            let colon_name = callee.replace("__", "::");
            if colon_name != *callee {
                refs.insert(colon_name);
            }
            let mangled_name = callee.replace("::", "__");
            if mangled_name != *callee {
                refs.insert(mangled_name);
            }
            if let Some(Expr::Identifier(recv)) = args.first() {
                refs.insert(format!("{}::{}", recv, callee));
                refs.insert(format!("{}__{}", recv, callee));
            }
            for arg in args {
                collect_references_in_expr_scoped(arg, locals, refs, called_methods);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_references_in_expr_scoped(left, locals, refs, called_methods);
            collect_references_in_expr_scoped(right, locals, refs, called_methods);
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => {
            collect_references_in_expr_scoped(expr, locals, refs, called_methods);
        }
        Expr::Array(elements) => {
            for elem in elements {
                collect_references_in_expr_scoped(elem, locals, refs, called_methods);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_references_in_expr_scoped(array, locals, refs, called_methods);
            collect_references_in_expr_scoped(index, locals, refs, called_methods);
        }
        Expr::FieldAccess { object, field } | Expr::OptionalFieldAccess { object, field } => {
            called_methods.insert(bare_name(field).to_string());
            called_methods.insert(field.clone());
            if let Expr::Identifier(mod_name) = &**object {
                refs.insert(format!("{}::{}", mod_name, field));
                refs.insert(format!("{}__{}", mod_name, field));
                refs.insert(field.clone());
            }
            collect_references_in_expr_scoped(object, locals, refs, called_methods);
        }
        Expr::StructInit { name, fields } => {
            refs.insert(name.clone());
            for (_, val) in fields {
                collect_references_in_expr_scoped(val, locals, refs, called_methods);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_references_in_expr_scoped(k, locals, refs, called_methods);
                collect_references_in_expr_scoped(v, locals, refs, called_methods);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                collect_references_in_expr_scoped(part, locals, refs, called_methods);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_references_in_expr_scoped(condition, locals, refs, called_methods);
            collect_references_in_expr_scoped(then_branch, locals, refs, called_methods);
            collect_references_in_expr_scoped(else_branch, locals, refs, called_methods);
        }
        Expr::NullCoalesce { value, default } => {
            collect_references_in_expr_scoped(value, locals, refs, called_methods);
            collect_references_in_expr_scoped(default, locals, refs, called_methods);
        }
        Expr::TypeCheck { expr, target, .. } | Expr::Cast { expr, target } => {
            collect_type_names(target, refs);
            collect_references_in_expr_scoped(expr, locals, refs, called_methods);
        }
        _ => {}
    }
}

fn collect_struct_inits_in_stmt(stmt: &Stmt, inits: &mut HashSet<String>) {
    match stmt.inner_stmt() {
        Stmt::Function { body, .. } => {
            for s in body {
                collect_struct_inits_in_stmt(s, inits);
            }
        }
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                collect_struct_inits_in_stmt(s, inits);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    collect_struct_inits_in_stmt(s, inits);
                }
            }
        }
        Stmt::While { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. }
        | Stmt::Repeat { body } => {
            for s in body {
                collect_struct_inits_in_stmt(s, inits);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_struct_inits_in_stmt(s, inits);
            }
            for s in catch_block {
                collect_struct_inits_in_stmt(s, inits);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    collect_struct_inits_in_stmt(s, inits);
                }
            }
        }
        Stmt::Expr(e)
        | Stmt::Say(e)
        | Stmt::Let { value: e, .. }
        | Stmt::Assign { value: e, .. }
        | Stmt::Return(Some(e)) => {
            collect_struct_inits_in_expr(e, inits);
        }
        _ => {}
    }
}

fn collect_struct_inits_in_expr(expr: &Expr, inits: &mut HashSet<String>) {
    match expr {
        Expr::StructInit { name, fields } => {
            inits.insert(name.clone());
            inits.insert(bare_name(name).to_string());
            for (_, val) in fields {
                collect_struct_inits_in_expr(val, inits);
            }
        }
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            for a in args {
                collect_struct_inits_in_expr(a, inits);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_struct_inits_in_expr(left, inits);
            collect_struct_inits_in_expr(right, inits);
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => {
            collect_struct_inits_in_expr(expr, inits);
        }
        Expr::Array(elements) | Expr::InterpolatedString(elements) => {
            for e in elements {
                collect_struct_inits_in_expr(e, inits);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_struct_inits_in_expr(array, inits);
            collect_struct_inits_in_expr(index, inits);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_struct_inits_in_expr(object, inits);
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_struct_inits_in_expr(k, inits);
                collect_struct_inits_in_expr(v, inits);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_struct_inits_in_expr(condition, inits);
            collect_struct_inits_in_expr(then_branch, inits);
            collect_struct_inits_in_expr(else_branch, inits);
        }
        Expr::NullCoalesce { value, default } => {
            collect_struct_inits_in_expr(value, inits);
            collect_struct_inits_in_expr(default, inits);
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
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(42))],
                },
                Stmt::Function {
                    name: "dead_fn".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(99))],
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
                    type_params: vec![],
                    attributes: vec![],
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
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(1))],
                },
                Stmt::Function {
                    name: "unreachable_fn".into(),
                    type_params: vec![],
                    attributes: vec![],
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
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "register_cb".into(),
                    type_params: vec![],
                    attributes: vec![],
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
                    attributes: vec![],
                },
                Stmt::StructDef {
                    name: "UnusedPoint".into(),
                    fields: vec!["z".into()],
                    field_types: vec![None],
                    defaults: vec![None],
                    attributes: vec![],
                },
                Stmt::Expr(Expr::StructInit {
                    name: "ActivePoint".into(),
                    fields: vec![("x".into(), Expr::Number(1)), ("y".into(), Expr::Number(2))],
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
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "lib_b".into(),
                    type_params: vec![],
                    attributes: vec![],
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

    #[test]
    fn test_dce_library_roots_pub_not_suite() {
        let program = Program {
            statements: vec![
                Stmt::Pub(Box::new(Stmt::Function {
                    name: "exported_api".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Expr(Expr::Call {
                        name: "internal_used_helper".into(),
                        args: vec![],
                    })],
                })),
                Stmt::Function {
                    name: "internal_used_helper".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "internal_dead_helper".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "test_feature".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Let {
                    name: "config".into(),
                    type_ann: None,
                    value: Expr::Number(100),
                },
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

        assert!(fn_names.contains(&"exported_api".to_string()));
        assert!(fn_names.contains(&"internal_used_helper".to_string()));
        // Unreferenced `test_*` functions are suite entries, not library
        // roots: only the test/bench pipeline (via synthesized calls) keeps
        // them alive.
        assert!(!fn_names.contains(&"test_feature".to_string()));
        assert!(!fn_names.contains(&"internal_dead_helper".to_string()));
    }

    fn suite_fn(name: &str, attributes: Vec<Attribute>) -> Stmt {
        Stmt::Function {
            name: name.into(),
            type_params: vec![],
            attributes,
            params: vec![],
            param_types: vec![],
            return_type: None,
            defaults: vec![],
            body: vec![Stmt::Say(Expr::Number(1))],
        }
    }

    fn test_attr() -> Attribute {
        Attribute {
            name: "test".into(),
            args: vec![],
        }
    }

    fn pruned_fn_names(program: &Program) -> Vec<String> {
        eliminate_dead_code(program)
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::Function { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn test_dce_pure_library_prunes_suite_entries() {
        // Mirrors the issue repro: a main-less library keeps helpers but
        // drops `test_*`, lowered `__test_*` / `__bench_*` blocks, and
        // `@test`-attributed functions.
        let program = Program {
            statements: vec![
                suite_fn("helper", vec![]),
                suite_fn("test_keepme", vec![]),
                suite_fn("__test_block_keepme", vec![]),
                suite_fn("attr_keepme", vec![test_attr()]),
                suite_fn("__bench_bench_keepme", vec![]),
            ],
        };

        let fn_names = pruned_fn_names(&program);
        assert_eq!(fn_names, vec!["helper"]);
    }

    #[test]
    fn test_dce_pure_library_keeps_referenced_suite_entry() {
        // A suite-flavored function that is actually called by kept code
        // must survive: pruning it would leave a dangling reference.
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "helper".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Expr(Expr::Call {
                        name: "__test_shared".into(),
                        args: vec![],
                    })],
                },
                suite_fn("__test_shared", vec![]),
                suite_fn("__test_orphan", vec![]),
            ],
        };

        let fn_names = pruned_fn_names(&program);
        assert!(fn_names.contains(&"helper".to_string()));
        assert!(fn_names.contains(&"__test_shared".to_string()));
        assert!(!fn_names.contains(&"__test_orphan".to_string()));
    }

    #[test]
    fn test_dce_no_main_with_top_level_prunes_suite_entries() {
        // Library with top-level statements but no main: suite entries are
        // not roots either and must be pruned when unreferenced.
        let program = Program {
            statements: vec![
                suite_fn("helper", vec![]),
                suite_fn("test_keepme", vec![]),
                suite_fn("__bench_bench_keepme", vec![]),
                Stmt::Expr(Expr::Call {
                    name: "helper".into(),
                    args: vec![],
                }),
            ],
        };

        let fn_names = pruned_fn_names(&program);
        assert!(fn_names.contains(&"helper".to_string()));
        assert!(!fn_names.contains(&"test_keepme".to_string()));
        assert!(!fn_names.contains(&"__bench_bench_keepme".to_string()));
    }

    #[test]
    fn test_dce_prunes_unreferenced_extern_functions_in_block() {
        let program = Program {
            statements: vec![
                Stmt::ExternBlock {
                    abi: "C".into(),
                    lib: Some("sqlite3".into()),
                    functions: vec![
                        ExternFnDecl {
                            name: "sqlite3_open".into(),
                            params: vec![],
                            return_type: Some("i32".into()),
                        },
                        ExternFnDecl {
                            name: "sqlite3_blob_read".into(),
                            params: vec![],
                            return_type: Some("i32".into()),
                        },
                    ],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Expr(Expr::Call {
                        name: "sqlite3_open".into(),
                        args: vec![],
                    })],
                },
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let mut extern_fns = Vec::new();
        for s in &pruned.statements {
            if let Stmt::ExternBlock { functions, .. } = s.inner_stmt() {
                for f in functions {
                    extern_fns.push(f.name.clone());
                }
            }
        }

        assert_eq!(extern_fns, vec!["sqlite3_open"]);
    }

    #[test]
    fn test_dce_prunes_entire_unreferenced_extern_block() {
        let program = Program {
            statements: vec![
                Stmt::ExternBlock {
                    abi: "C".into(),
                    lib: Some("unused_lib".into()),
                    functions: vec![
                        ExternFnDecl {
                            name: "unused_func1".into(),
                            params: vec![],
                            return_type: None,
                        },
                        ExternFnDecl {
                            name: "unused_func2".into(),
                            params: vec![],
                            return_type: None,
                        },
                    ],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(42))],
                },
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let has_extern = pruned
            .statements
            .iter()
            .any(|s| matches!(s.inner_stmt(), Stmt::ExternBlock { .. }));

        assert!(
            !has_extern,
            "Unused ExternBlock should be completely eliminated"
        );
    }

    #[test]
    fn test_dce_prunes_uncalled_struct_methods() {
        let program = Program {
            statements: vec![
                Stmt::StructDef {
                    name: "Point".into(),
                    fields: vec!["x".into(), "y".into()],
                    field_types: vec![None, None],
                    defaults: vec![None, None],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "Point__used_method".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(1))],
                },
                Stmt::Function {
                    name: "Point__dead_method".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(2))],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![
                        Stmt::Let {
                            name: "p".into(),
                            type_ann: None,
                            value: Expr::StructInit {
                                name: "Point".into(),
                                fields: vec![],
                            },
                        },
                        Stmt::Expr(Expr::Call {
                            name: "used_method".into(),
                            args: vec![Expr::Identifier("p".into())],
                        }),
                    ],
                },
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

        assert!(fn_names.contains(&"main".to_string()));
        assert!(fn_names.contains(&"Point__used_method".to_string()));
        assert!(!fn_names.contains(&"Point__dead_method".to_string()));
    }

    #[test]
    fn test_dce_primitive_type_ann_does_not_pull_method() {
        let program = Program {
            statements: vec![
                Stmt::StructDef {
                    name: "Rng".into(),
                    fields: vec!["seed".into()],
                    field_types: vec![None],
                    defaults: vec![None],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "Rng__int".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(1))],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Let {
                        name: "x".into(),
                        type_ann: Some("int".into()),
                        value: Expr::Number(42),
                    }],
                },
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

        assert_eq!(fn_names, vec!["main"]);
        let has_rng = pruned
            .statements
            .iter()
            .any(|s| matches!(s.inner_stmt(), Stmt::StructDef { name, .. } if name == "Rng"));
        assert!(!has_rng);
    }

    #[test]
    fn test_dce_local_param_does_not_pull_uncalled_bare_method() {
        let program = Program {
            statements: vec![
                Stmt::StructDef {
                    name: "UrlBuilder".into(),
                    fields: vec!["path".into()],
                    field_types: vec![None],
                    defaults: vec![None],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "UrlBuilder__path".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(1))],
                },
                Stmt::Function {
                    name: "helper".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["path".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Identifier("path".into()))],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Expr(Expr::Call {
                        name: "helper".into(),
                        args: vec![Expr::String("test".into())],
                    })],
                },
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

        assert!(fn_names.contains(&"main".to_string()));
        assert!(fn_names.contains(&"helper".to_string()));
        assert!(!fn_names.contains(&"UrlBuilder__path".to_string()));
    }

    #[test]
    fn test_dce_interface_methods_preserved_for_type_check_and_unused_pruned() {
        let program = Program {
            statements: vec![
                Stmt::InterfaceDef {
                    name: "Shape".into(),
                    methods: vec![InterfaceMethod {
                        name: "area".into(),
                        params: vec!["self".into()],
                        param_types: vec![],
                        return_type: None,
                    }],
                    embedded: vec![],
                },
                Stmt::InterfaceDef {
                    name: "UnusedInterface".into(),
                    methods: vec![InterfaceMethod {
                        name: "unused_m".into(),
                        params: vec!["self".into()],
                        param_types: vec![],
                        return_type: None,
                    }],
                    embedded: vec![],
                },
                Stmt::StructDef {
                    name: "Circle".into(),
                    fields: vec!["radius".into()],
                    field_types: vec![None],
                    defaults: vec![None],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "Circle__area".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(100))],
                },
                Stmt::Function {
                    name: "Circle__dead".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec!["self".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(0))],
                },
                Stmt::Function {
                    name: "main".into(),
                    type_params: vec![],
                    attributes: vec![],
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![
                        Stmt::Let {
                            name: "c".into(),
                            type_ann: None,
                            value: Expr::StructInit {
                                name: "Circle".into(),
                                fields: vec![],
                            },
                        },
                        Stmt::Expr(Expr::TypeCheck {
                            expr: Box::new(Expr::Identifier("c".into())),
                            target: "Shape".into(),
                            negated: false,
                        }),
                    ],
                },
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

        assert!(fn_names.contains(&"main".to_string()));
        assert!(fn_names.contains(&"Circle__area".to_string()));
        assert!(!fn_names.contains(&"Circle__dead".to_string()));

        let iface_names: Vec<String> = pruned
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::InterfaceDef { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(iface_names.contains(&"Shape".to_string()));
        assert!(!iface_names.contains(&"UnusedInterface".to_string()));
    }

    #[test]
    fn test_dce_aliased_module_call_preserved() {
        let code = "import \"std/fs\" as fs\nfunction foo() -> string\n    return fs.read_string(\"a.txt\")\nend\nsay foo()";
        let tokens = crate::lexer::Lexer::new(code).tokenize().unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let mut program = parser.parse().unwrap();
        crate::parser::resolve_imports(
            &mut program,
            std::path::Path::new("."),
            &crate::parser::CfgContext::host(),
        )
        .unwrap();
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
        assert!(fn_names.iter().any(|f| f.contains("read_string")));
    }
}
