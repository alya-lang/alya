pub mod analysis;
pub mod arch;
pub mod context;
mod expr;
pub mod kinds;
pub mod peephole;
pub mod runtime;
mod say;
mod stmt;
pub mod target;
#[cfg(test)]
mod tests;

pub use target::{Architecture, OperatingSystem};

use crate::ast::*;
use analysis::ProgramInference;
use context::{CodeGenContext, VarType};

#[derive(Debug, Default, Clone)]
pub struct PipelineProfile {
    pub d_call_index: std::time::Duration,
    pub d_dce: std::time::Duration,
    pub d_inference: std::time::Duration,
    pub d_codegen: std::time::Duration,
    pub original_stmts: usize,
    pub pruned_stmts: usize,
}

pub struct CodeGen {
    pub(crate) arch: Architecture,
    pub(crate) os: OperatingSystem,
    pub(crate) output: String,
    pub(crate) ctx: CodeGenContext,
    pub profile: PipelineProfile,
    pub no_std: bool,
    pub mem_trace: bool,
}

fn collect_expr_identifiers(expr: &Expr, idents: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Identifier(id) => {
            idents.insert(id.clone());
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_identifiers(left, idents);
            collect_expr_identifiers(right, idents);
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => {
            collect_expr_identifiers(expr, idents);
        }
        Expr::ForceUnwrap(inner) => {
            collect_expr_identifiers(inner, idents);
        }
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            for arg in args {
                collect_expr_identifiers(arg, idents);
            }
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            for item in items {
                collect_expr_identifiers(item, idents);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_expr_identifiers(array, idents);
            collect_expr_identifiers(index, idents);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_expr_identifiers(object, idents);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_expr_identifiers(val, idents);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                collect_expr_identifiers(k, idents);
                collect_expr_identifiers(v, idents);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_identifiers(condition, idents);
            collect_expr_identifiers(then_branch, idents);
            collect_expr_identifiers(else_branch, idents);
        }
        Expr::NullCoalesce { value, default } => {
            collect_expr_identifiers(value, idents);
            collect_expr_identifiers(default, idents);
        }
        _ => {}
    }
}

fn collect_called_names_expr(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Call { name, args } => {
            out.push(name.clone());
            for arg in args {
                collect_called_names_expr(arg, out);
            }
        }
        Expr::OptionalCall { callee, args } => {
            out.push(callee.clone());
            for arg in args {
                collect_called_names_expr(arg, out);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_called_names_expr(left, out);
            collect_called_names_expr(right, out);
        }
        Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => {
            collect_called_names_expr(expr, out);
        }
        Expr::ForceUnwrap(inner) => {
            collect_called_names_expr(inner, out);
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            for item in items {
                collect_called_names_expr(item, out);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_called_names_expr(array, out);
            collect_called_names_expr(index, out);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_called_names_expr(object, out);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_called_names_expr(val, out);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                collect_called_names_expr(k, out);
                collect_called_names_expr(v, out);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_called_names_expr(condition, out);
            collect_called_names_expr(then_branch, out);
            collect_called_names_expr(else_branch, out);
        }
        Expr::NullCoalesce { value, default } => {
            collect_called_names_expr(value, out);
            collect_called_names_expr(default, out);
        }
        _ => {}
    }
}

fn collect_called_names_stmt(stmt: &Stmt, out: &mut Vec<String>) {
    match stmt.inner_stmt() {
        Stmt::Expr(e) | Stmt::Say(e) | Stmt::Return(Some(e)) | Stmt::Throw(Some(e)) => {
            collect_called_names_expr(e, out);
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
            collect_called_names_expr(value, out);
        }
        Stmt::Assign { value, .. } => {
            collect_called_names_expr(value, out);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_called_names_expr(array, out);
            collect_called_names_expr(index, out);
            collect_called_names_expr(value, out);
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_called_names_expr(object, out);
            collect_called_names_expr(value, out);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_called_names_expr(condition, out);
            for s in then_block {
                collect_called_names_stmt(s, out);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    collect_called_names_stmt(s, out);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_called_names_expr(condition, out);
            for s in body {
                collect_called_names_stmt(s, out);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                collect_called_names_stmt(s, out);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            collect_called_names_expr(start, out);
            collect_called_names_expr(end, out);
            for s in body {
                collect_called_names_stmt(s, out);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            collect_called_names_expr(iterable, out);
            for s in body {
                collect_called_names_stmt(s, out);
            }
        }
        Stmt::Defer(inner) => {
            collect_called_names_stmt(inner, out);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_called_names_stmt(s, out);
            }
            for s in catch_block {
                collect_called_names_stmt(s, out);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    collect_called_names_stmt(s, out);
                }
            }
        }
        Stmt::Function { body, .. } => {
            for s in body {
                collect_called_names_stmt(s, out);
            }
        }
        Stmt::Pub(inner) => {
            collect_called_names_stmt(inner, out);
        }
        _ => {}
    }
}

fn bare_variants(name: &str) -> Vec<String> {
    let mut keys = vec![name.to_string()];
    let ns_bare = name.rsplit("::").next().unwrap_or(name);
    if ns_bare != name {
        keys.push(ns_bare.to_string());
    }
    let bare = ns_bare.rsplit("__").next().unwrap_or(ns_bare);
    if bare != ns_bare {
        keys.push(bare.to_string());
    }
    keys
}

/// Emission order for the return-tag protocol (alya-lang/alya#55-C):
/// callees before callers, so forward references see their callee's
/// marker. Tarjan SCCs over the call graph (edges caller -> callee);
/// cyclic groups (recursion) keep source order and today's legacy
/// behavior. Best-effort edges only: a missed edge is a missed
/// optimization, never a miscompile — marker insertion still runs on
/// live vars at generation time, unchanged.
pub(crate) fn order_functions_callee_first(functions: Vec<&Stmt>) -> Vec<&Stmt> {
    let names: Vec<String> = functions
        .iter()
        .map(|s| match s.inner_stmt() {
            Stmt::Function { name, .. } => name.clone(),
            _ => String::new(),
        })
        .collect();
    let mut key_to_idxs: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, name) in names.iter().enumerate() {
        for key in bare_variants(name) {
            key_to_idxs.entry(key).or_default().push(i);
        }
    }
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); functions.len()];
    for (i, func) in functions.iter().enumerate() {
        if let Stmt::Function { body, .. } = func.inner_stmt() {
            let mut called = Vec::new();
            for s in body {
                collect_called_names_stmt(s, &mut called);
            }
            let mut seen = std::collections::HashSet::new();
            for name in called {
                for key in bare_variants(&name) {
                    if let Some(idxs) = key_to_idxs.get(&key) {
                        for &j in idxs {
                            if seen.insert(j) {
                                edges[i].push(j);
                            }
                        }
                    }
                }
            }
        }
    }
    // Tarjan SCC (recursive; graphs are small). Completion order is
    // sinks-first, i.e. callees before callers along every edge.
    struct Tarjan {
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        sccs: Vec<Vec<usize>>,
    }
    fn strongconnect(t: &mut Tarjan, v: usize, edges: &[Vec<usize>]) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on_stack[v] = true;
        for &w in &edges[v] {
            if t.index[w].is_none() {
                strongconnect(t, w, edges);
                t.low[v] = t.low[v].min(t.low[w]);
            } else if t.on_stack[w] {
                t.low[v] = t.low[v].min(t.index[w].unwrap_or(usize::MAX));
            }
        }
        if t.low[v] == t.index[v].unwrap_or(usize::MAX) {
            let mut scc = Vec::new();
            while let Some(w) = t.stack.pop() {
                t.on_stack[w] = false;
                scc.push(w);
                if w == v {
                    break;
                }
            }
            scc.sort_unstable();
            t.sccs.push(scc);
        }
    }
    let n = functions.len();
    let mut t = Tarjan {
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        sccs: Vec::new(),
    };
    for v in 0..n {
        if t.index[v].is_none() {
            strongconnect(&mut t, v, &edges);
        }
    }
    let mut ordered = Vec::with_capacity(n);
    for scc in t.sccs {
        for i in scc {
            ordered.push(functions[i]);
        }
    }
    ordered
}

fn collect_stmt_identifiers(stmt: &Stmt, idents: &mut std::collections::HashSet<String>) {
    match stmt.inner_stmt() {
        Stmt::Expr(e) | Stmt::Say(e) | Stmt::Return(Some(e)) | Stmt::Throw(Some(e)) => {
            collect_expr_identifiers(e, idents);
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
            collect_expr_identifiers(value, idents);
        }
        Stmt::Assign { name, value } => {
            idents.insert(name.clone());
            collect_expr_identifiers(value, idents);
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_expr_identifiers(object, idents);
            collect_expr_identifiers(value, idents);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_expr_identifiers(array, idents);
            collect_expr_identifiers(index, idents);
            collect_expr_identifiers(value, idents);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_identifiers(condition, idents);
            for s in then_block {
                collect_stmt_identifiers(s, idents);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    collect_stmt_identifiers(s, idents);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_expr_identifiers(condition, idents);
            for s in body {
                collect_stmt_identifiers(s, idents);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                collect_stmt_identifiers(s, idents);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            collect_expr_identifiers(start, idents);
            collect_expr_identifiers(end, idents);
            for s in body {
                collect_stmt_identifiers(s, idents);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            collect_expr_identifiers(iterable, idents);
            for s in body {
                collect_stmt_identifiers(s, idents);
            }
        }
        Stmt::Defer(inner) => {
            collect_stmt_identifiers(inner, idents);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_stmt_identifiers(s, idents);
            }
            for s in catch_block {
                collect_stmt_identifiers(s, idents);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    collect_stmt_identifiers(s, idents);
                }
            }
        }
        Stmt::Pub(inner) => {
            collect_stmt_identifiers(inner, idents);
        }
        _ => {}
    }
}

fn collect_local_stmt_vars(stmts: &[Stmt], vars: &mut std::collections::HashSet<String>) {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Let { name, .. } => {
                vars.insert(name.clone());
            }
            Stmt::For { var, .. } => {
                vars.insert(var.clone());
            }
            Stmt::ForEach { var, value_var, .. } => {
                vars.insert(var.clone());
                if let Some(ref v) = value_var {
                    vars.insert(v.clone());
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_local_stmt_vars(then_block, vars);
                if let Some(eb) = else_block {
                    collect_local_stmt_vars(eb, vars);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body, .. } => {
                collect_local_stmt_vars(body, vars);
            }
            _ => {}
        }
    }
}

impl CodeGen {
    pub fn new(arch: Architecture, os: OperatingSystem) -> Self {
        Self {
            arch,
            os,
            output: String::new(),
            ctx: CodeGenContext::new(),
            profile: PipelineProfile::default(),
            no_std: false,
            mem_trace: false,
        }
    }

    #[inline]
    pub(crate) fn temp_offset(&self) -> i32 {
        match self.arch {
            crate::codegen::target::Architecture::ARM64 => 16,
            _ => 8,
        }
    }

    pub fn generate_program(&mut self, program: &Program) {
        let mut resolved_prog = program.clone();
        // Resolve once: every recording site short-circuits on false.
        self.ctx.tag_stats.enabled = std::env::var("ALYA_TAG_STATS").is_ok();
        crate::parser::enums::resolve_enums(&mut resolved_prog);
        let _ = crate::parser::constants::resolve_and_validate_constants(&mut resolved_prog);
        crate::parser::generics::resolve_generics(&mut resolved_prog);
        crate::parser::dynspec::resolve_dynspec(&mut resolved_prog);
        // Lexical nested functions become top-level qualified functions
        // (alya-lang/alya#99) before DCE/inference see the program.
        crate::codegen::analysis::nested::hoist_nested_functions(&mut resolved_prog);
        let program = &resolved_prog;

        let original_stmts = program.statements.len();
        let t_dce = std::time::Instant::now();
        let pruned_prog = analysis::eliminate_dead_code(program);
        let d_dce = t_dce.elapsed();
        let pruned_stmts = pruned_prog.statements.len();
        let program = &pruned_prog;

        // Collect all enum definitions first
        for stmt in &program.statements {
            if let Stmt::EnumDef { name, .. } = stmt.inner_stmt() {
                self.ctx.enums.insert(name.clone());
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    self.ctx.enums.insert(bare.to_string());
                }
            }
        }

        // Collect all struct definitions first
        for stmt in &program.statements {
            if let Stmt::StructDef {
                name,
                fields,
                field_types,
                defaults,
                ..
            } = stmt.inner_stmt()
            {
                self.ctx.structs.insert(
                    name.clone(),
                    context::StructDefInfo {
                        name: name.clone(),
                        fields: fields.clone(),
                        field_types: field_types.clone(),
                        defaults: defaults.clone(),
                    },
                );
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    self.ctx.structs.insert(
                        bare.to_string(),
                        context::StructDefInfo {
                            name: bare.to_string(),
                            fields: fields.clone(),
                            field_types: field_types.clone(),
                            defaults: defaults.clone(),
                        },
                    );
                }
                for (f, ft) in fields.iter().zip(field_types.iter()) {
                    if let Some(t) = ft {
                        if t == "string" || t == "str" {
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", name, f),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", bare, f),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", f),
                                VarType::StringOffset(0),
                            );
                        } else if t == "float" || t == "f64" || t == "f32" {
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", name, f),
                                VarType::Float(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", bare, f),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", f), VarType::Float(0));
                        } else if t == "array" || t.ends_with("[]") {
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", name, f),
                                VarType::Array(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", bare, f),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", f), VarType::Array(0));
                            if t == "string[]" || t == "str[]" {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_str:{}.{}", name, f),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_str:{}.{}", bare, f),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_str:{}", f),
                                    VarType::Number(0),
                                );
                            } else if t == "float[]" || t == "f64[]" || t == "f32[]" {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_flt:{}.{}", name, f),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_flt:{}.{}", bare, f),
                                    VarType::Number(0),
                                );
                            } else if crate::codegen::analysis::is_int_array_annotation(t) {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_int:{}.{}", name, f),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_int:{}.{}", bare, f),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_int:{}", f),
                                    VarType::Number(0),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Collect all interface definitions
        for stmt in &program.statements {
            if let Stmt::InterfaceDef {
                name,
                methods,
                embedded,
            } = stmt.inner_stmt()
            {
                self.ctx.interfaces.insert(
                    name.clone(),
                    context::InterfaceDefInfo {
                        name: name.clone(),
                        methods: methods.clone(),
                        embedded: embedded.clone(),
                    },
                );
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    self.ctx.interfaces.insert(
                        bare.to_string(),
                        context::InterfaceDefInfo {
                            name: bare.to_string(),
                            methods: methods.clone(),
                            embedded: embedded.clone(),
                        },
                    );
                }
            }
        }

        let (inference, (d_call_index, d_inference)) =
            ProgramInference::analyze_with_timing(program);
        for s in &inference.known_strings {
            if s.starts_with("map_field_str:")
                || s.starts_with("map_str:")
                || s.starts_with("map_nonstr:")
                || s.starts_with("fn_ret_str:")
                || s.starts_with("fn_ret_str_arr:")
                || s.starts_with("fn_ret_tuple_str:")
                || s.starts_with("tuple_elem_str:")
                || s.starts_with("struct_field_str:")
                || s.starts_with("fn_param_str:")
                || s.starts_with("fn_param_str_arr:")
            {
                self.ctx
                    .variables
                    .insert(s.clone(), VarType::StringOffset(0));
            }
            // Mixed-literal struct fields: presence-only sentinels consulted
            // by codegen when emitting global field markers. Never queried
            // as a value type; the key shape is unmistakable.
            if s.starts_with("struct_field_mixed:") {
                self.ctx.variables.insert(s.clone(), VarType::Number(0));
            }
        }

        for s in &inference.known_floats {
            if s.starts_with("fn_ret_flt:")
                || s.starts_with("fn_ret_flt_ann:")
                || s.starts_with("fn_ret_tuple_flt:")
                || s.starts_with("struct_field_flt:")
                || s.starts_with("tuple_elem_flt:")
                // Call-site width queries: seeded like the other `fn_*` markers and
                // carried across function scopes (see `enter_function`).
                // `fn_param_flt_arr:` is excluded on purpose (float-array
                // params are pointers).
                || s.starts_with("fn_param_flt:")
            {
                self.ctx.variables.insert(s.clone(), VarType::Float(0));
            }
            // alya-lang/alya#50: push-built float arrays earn their
            // marking in inference; codegen reads consult it via
            // `is_float_array` exactly like literal-derived marks
            // (presence-only, same `Number(0)` shape as assign.rs).
            // `arr_nonflt:` vetoes (#95) travel alongside so mixed
            // arrays degrade to slot-kind dispatch. `arr_flt_ann:`
            // marks explicit `float[]` annotations whose conversion
            // semantics survive int pushes.
            if s.starts_with("arr_is_flt:")
                || s.starts_with("arr_nonflt:")
                || s.starts_with("arr_flt_ann:")
            {
                self.ctx.variables.insert(s.clone(), VarType::Number(0));
            }
        }

        for m in &inference.known_maps {
            if m.starts_with("fn_ret_map:") {
                self.ctx.variables.insert(m.clone(), VarType::Map(0));
            }
        }

        for a in &inference.known_arrays {
            if a.starts_with("fn_ret_arr:")
                || a.starts_with("struct_field_arr:")
                || a.starts_with("fn_param_arr:")
            {
                self.ctx.variables.insert(a.clone(), VarType::Array(0));
            }
        }

        // Ambiguous bare sentinels for recording sites below (#101):
        // qualified definitions sharing a bare skip bare inserts, so
        // collided keys carry only the simple owner's claims.
        // Presence-only; the key shape matches no value query.
        for bare in crate::codegen::analysis::ambiguous_bares_of_program(program) {
            self.ctx.variables.insert(
                crate::codegen::analysis::ambiguous_bare_key(&bare),
                VarType::Number(0),
            );
        }

        for stmt in &program.statements {
            if let Stmt::Function {
                name,
                params,
                param_types,
                return_type,
                defaults,
                ..
            } = stmt.inner_stmt()
            {
                self.ctx.functions.insert(name.clone());
                let mangled_fn = crate::codegen::arch::control::mangle_symbol_name(name);
                self.ctx.functions.insert(mangled_fn);
                let ns_bare = name.rsplit("::").next().unwrap_or(name);
                let bare = if let Some((prefix, _)) = ns_bare.split_once("__") {
                    if self.ctx.structs.contains_key(prefix) || self.ctx.enums.contains(prefix) {
                        ns_bare
                    } else {
                        ns_bare.rsplit("__").next().unwrap_or(ns_bare)
                    }
                } else {
                    ns_bare
                };
                // Arity for builtin-shadowing decisions (assert/assert_eq):
                // required = params without defaults (defaults align with
                // params; a short defaults vec means all required).
                let total = params.len();
                let defaulted = defaults.iter().take(total).filter(|d| d.is_some()).count();
                let required = total.saturating_sub(defaulted);
                self.ctx.fn_arities.insert(name.clone(), (required, total));
                if bare != name {
                    self.ctx
                        .fn_arities
                        .insert(bare.to_string(), (required, total));
                }

                for (i, ptype) in param_types.iter().enumerate() {
                    if let Some(t) = ptype {
                        let bare_base = t.split('[').next().unwrap_or(t);
                        let b = bare_base.rsplit("::").next().unwrap_or(bare_base);
                        let b = b.rsplit("__").next().unwrap_or(b);
                        let iface_name = if self.ctx.interfaces.contains_key(bare_base) {
                            Some(bare_base.to_string())
                        } else if self.ctx.interfaces.contains_key(b) {
                            Some(b.to_string())
                        } else {
                            None
                        };
                        if let Some(iname) = iface_name {
                            self.ctx.variables.insert(
                                format!("fn_param_interface:{}:{}", name, i),
                                VarType::Interface {
                                    interface_name: iname.clone(),
                                    offset: 0,
                                },
                            );
                            if bare != name
                                // #101: bare markers need a single owner.
                                && crate::codegen::analysis::may_record_bare_vars(
                                    &self.ctx.variables,
                                    name,
                                    bare,
                                )
                            {
                                self.ctx.variables.insert(
                                    format!("fn_param_interface:{}:{}", bare, i),
                                    VarType::Interface {
                                        interface_name: iname,
                                        offset: 0,
                                    },
                                );
                            }
                        }
                    }
                }

                let explicit_struct = return_type.as_deref().and_then(|rt| {
                    let bare_rt = rt.rsplit("::").next().unwrap_or(rt);
                    let bare_rt = bare_rt.rsplit("__").next().unwrap_or(bare_rt);
                    if self.ctx.structs.contains_key(rt) {
                        Some(rt.to_string())
                    } else if self.ctx.structs.contains_key(bare_rt) {
                        Some(bare_rt.to_string())
                    } else {
                        None
                    }
                });

                if return_type.as_deref() != Some("any") {
                    if let Some(sname) = explicit_struct
                        .or_else(|| inference.infer_function_return_struct_type(name))
                    {
                        self.ctx.variables.insert(
                            format!("fn_ret_struct:{}", name),
                            VarType::Struct {
                                struct_name: sname.clone(),
                                offset: 0,
                            },
                        );
                        let colon_name = name.replace("__", "::");
                        self.ctx.variables.insert(
                            format!("fn_ret_struct:{}", colon_name),
                            VarType::Struct {
                                struct_name: sname.clone(),
                                offset: 0,
                            },
                        );
                        let mangled_name = name.replace("::", "__");
                        self.ctx.variables.insert(
                            format!("fn_ret_struct:{}", mangled_name),
                            VarType::Struct {
                                struct_name: sname.clone(),
                                offset: 0,
                            },
                        );
                        if bare != name
                            // #101: bare markers need a single owner.
                            && crate::codegen::analysis::may_record_bare_vars(
                                &self.ctx.variables,
                                name,
                                bare,
                            )
                        {
                            self.ctx.variables.insert(
                                format!("fn_ret_struct:{}", bare),
                                VarType::Struct {
                                    struct_name: sname,
                                    offset: 0,
                                },
                            );
                        }
                    }
                }

                // Freshness markers for temp-ownership release: when every
                // return yields a freshly-owned value, callers may drop
                // the temp (alya-lang/alya#79). Seeded with the same four
                // spellings as the struct markers above.
                if let crate::ast::Stmt::Function { body, .. } = stmt.inner_stmt() {
                    let struct_names: std::collections::HashSet<String> =
                        self.ctx.structs.keys().cloned().collect();
                    if crate::codegen::analysis::fn_returns_fresh_value(body, &struct_names) {
                        let colon_name = name.replace("__", "::");
                        let mangled_name = name.replace("::", "__");
                        let mut keys = vec![
                            format!("fn_ret_fresh:{}", name),
                            format!("fn_ret_fresh:{}", colon_name),
                            format!("fn_ret_fresh:{}", mangled_name),
                        ];
                        // #101: bare markers need a single owner.
                        if crate::codegen::analysis::may_record_bare_vars(
                            &self.ctx.variables,
                            name,
                            bare,
                        ) {
                            keys.push(format!("fn_ret_fresh:{}", bare));
                        }
                        for key in keys {
                            self.ctx.variables.insert(key, VarType::Number(0));
                        }
                    }
                }

                if let Some(rt) = return_type.as_deref() {
                    if rt == "array" || rt.ends_with("[]") {
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_arr:{}", name), VarType::Array(0));
                        // #101: bare markers need a single owner.
                        if crate::codegen::analysis::may_record_bare_vars(
                            &self.ctx.variables,
                            name,
                            bare,
                        ) {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_arr:{}", bare), VarType::Array(0));
                        }
                        let colon_name = name.replace("__", "::");
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_arr:{}", colon_name), VarType::Array(0));
                        let mangled_name = name.replace("::", "__");
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_arr:{}", mangled_name), VarType::Array(0));
                    }
                    // Explicit integer return annotations are recorded so
                    // that method-result classification can authoritatively
                    // rule out `string` for colliding bare names (e.g. a
                    // `Box__touch(...) -> int` method vs a `touch(path)`
                    // string helper). Bare form is deliberately NOT recorded
                    // to avoid reintroducing the same collisions.
                    let rt_base = rt.rsplit("::").next().unwrap_or(rt);
                    let rt_base = rt_base.rsplit("__").next().unwrap_or(rt_base);
                    if matches!(
                        rt_base,
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
                    ) {
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_int:{}", name), VarType::Number(0));
                        let colon_name = name.replace("__", "::");
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_int:{}", colon_name), VarType::Number(0));
                        let mangled_name = name.replace("::", "__");
                        self.ctx
                            .variables
                            .insert(format!("fn_ret_int:{}", mangled_name), VarType::Number(0));
                    }
                }
            }
        }

        for ((fname, idx), sname) in &inference.struct_inf.fn_tuple_returns {
            let colon_name = fname.replace("__", "::");
            let mangled_name = fname.replace("::", "__");
            let bare = crate::codegen::analysis::inference::structs::resolve_func_bare(
                fname,
                &inference.struct_inf.struct_names,
            );
            let mut keys = vec![
                format!("fn_ret_tuple_struct:{}:{}", fname, idx),
                format!("fn_ret_tuple_struct:{}:{}", colon_name, idx),
                format!("fn_ret_tuple_struct:{}:{}", mangled_name, idx),
            ];
            // #101: bare markers need a single owner.
            if crate::codegen::analysis::may_record_bare_vars(&self.ctx.variables, fname, bare) {
                keys.push(format!("fn_ret_tuple_struct:{}:{}", bare, idx));
            }
            for key in keys {
                self.ctx.variables.insert(
                    key,
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: 0,
                    },
                );
            }
        }

        // Compute virtual method tables (VTables) for structs implicitly satisfying interfaces
        for (sname, sdef) in &self.ctx.structs {
            let bare_sdef = sdef.name.rsplit("::").next().unwrap_or(&sdef.name);
            let bare_sdef = bare_sdef.rsplit("__").next().unwrap_or(bare_sdef);
            for (iname, idef) in &self.ctx.interfaces {
                let bare_idef = idef.name.rsplit("::").next().unwrap_or(&idef.name);
                let bare_idef = bare_idef.rsplit("__").next().unwrap_or(bare_idef);
                let flattened = get_interface_flattened_methods(&idef.name, &self.ctx.interfaces);
                if flattened.is_empty() {
                    continue;
                }
                let satisfies = flattened.iter().all(|m| {
                    let c1 = format!("{}__{}", sdef.name, m.name);
                    let c2 = format!("{}__{}", bare_sdef, m.name);
                    self.ctx.functions.contains(&c1)
                        || self.ctx.functions.contains(&c2)
                        || self.ctx.functions.iter().any(|f| {
                            (f.ends_with(&format!("__{}", m.name))
                                || f.ends_with(&format!("::{}", m.name)))
                                && f.contains(bare_sdef)
                        })
                });
                if satisfies {
                    let vtable_label = format!("alya_vtable_{}_{}", bare_sdef, bare_idef);
                    self.ctx.vtables.insert(
                        (bare_sdef.to_string(), bare_idef.to_string()),
                        vtable_label.clone(),
                    );
                    self.ctx
                        .vtables
                        .insert((sname.clone(), iname.clone()), vtable_label);
                    for m in &flattened {
                        if m.return_type.as_deref() == Some("float")
                            || m.return_type.as_deref() == Some("f64")
                        {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_flt:{}", m.name), VarType::Float(0));
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_flt_ann:{}", m.name), VarType::Float(0));
                        } else if m.return_type.as_deref() == Some("string")
                            || m.return_type.as_deref() == Some("str")
                        {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_str:{}", m.name), VarType::StringOffset(0));
                        }
                    }
                }
            }
        }

        // Multi-level freshness inference for temp-ownership release (alya-lang/alya#81):
        // propagates freshness across call chains (e.g. g() returns f() where f() is fresh)
        // and seeds built-in container constructors.
        let struct_names: std::collections::HashSet<String> =
            self.ctx.structs.keys().cloned().collect();
        let fresh_fns = crate::codegen::analysis::infer_program_fresh_functions(
            &program.statements,
            &struct_names,
        );
        for f in &fresh_fns {
            self.ctx
                .variables
                .insert(format!("fn_ret_fresh:{}", f), VarType::Number(0));
        }

        let struct_inf = &inference.struct_inf;
        for ((sname, fname), inner_st) in &struct_inf.field_types {
            if self.ctx.structs.contains_key(inner_st) {
                self.ctx.variables.insert(
                    format!("struct_field_struct:{}.{}", sname, fname),
                    VarType::Struct {
                        struct_name: inner_st.clone(),
                        offset: 0,
                    },
                );
                let bare = sname.rsplit("::").next().unwrap_or(sname);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != sname {
                    self.ctx.variables.insert(
                        format!("struct_field_struct:{}.{}", bare, fname),
                        VarType::Struct {
                            struct_name: inner_st.clone(),
                            offset: 0,
                        },
                    );
                }
                self.ctx.variables.insert(
                    format!("struct_field_struct:{}", fname),
                    VarType::Struct {
                        struct_name: inner_st.clone(),
                        offset: 0,
                    },
                );
            }
        }

        // Collect all extern declarations
        for stmt in &program.statements {
            if let Stmt::ExternBlock {
                abi,
                lib,
                functions,
            } = stmt.inner_stmt()
            {
                if let Some(ref l) = lib {
                    self.ctx.extern_libs.insert(l.clone());
                }
                for f in functions {
                    let info = context::ExternFnInfo {
                        abi: abi.clone(),
                        lib: lib.clone(),
                        name: f.name.clone(),
                        return_type: f.return_type.clone(),
                        params_count: f.params.len(),
                        params: f.params.iter().map(|p| p.param_type.clone()).collect(),
                    };
                    self.ctx
                        .extern_functions
                        .insert(f.name.clone(), info.clone());
                    let bare = f.name.rsplit("::").next().unwrap_or(&f.name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if bare != f.name {
                        self.ctx.extern_functions.insert(bare.to_string(), info);
                    }
                    if let Some(ref ret_type) = f.return_type {
                        if ret_type == "str" || ret_type == "string" {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_str:{}", f.name), VarType::StringOffset(0));
                            // #101: bare markers need a single owner.
                            if bare != f.name
                                && crate::codegen::analysis::may_record_bare_vars(
                                    &self.ctx.variables,
                                    &f.name,
                                    bare,
                                )
                            {
                                self.ctx.variables.insert(
                                    format!("fn_ret_str:{}", bare),
                                    VarType::StringOffset(0),
                                );
                            }
                        } else if ret_type == "float" || ret_type == "f64" || ret_type == "f32" {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_flt:{}", f.name), VarType::Float(0));
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_flt_ann:{}", f.name), VarType::Float(0));
                            // #101: bare markers need a single owner.
                            if bare != f.name
                                && crate::codegen::analysis::may_record_bare_vars(
                                    &self.ctx.variables,
                                    &f.name,
                                    bare,
                                )
                            {
                                self.ctx
                                    .variables
                                    .insert(format!("fn_ret_flt:{}", bare), VarType::Float(0));
                                self.ctx
                                    .variables
                                    .insert(format!("fn_ret_flt_ann:{}", bare), VarType::Float(0));
                            }
                        }
                    }
                }
            }
        }

        let mut functions = Vec::new();
        let mut top_level = Vec::new();

        for stmt in &program.statements {
            match stmt.inner_stmt() {
                Stmt::Function { .. } => functions.push(stmt.inner_stmt()),
                _ => top_level.push(stmt),
            }
        }
        // Return-tag protocol (alya-lang/alya#55-C): emit callees
        // before callers so forward references see their marker.
        // Layout-only: every function still emits exactly once.
        let functions = order_functions_callee_first(functions);

        // Identify top-level variables used in functions/lambdas that need to be module globals
        let mut function_idents = std::collections::HashSet::new();
        for func in &functions {
            if let Stmt::Function { params, body, .. } = func {
                let mut local_vars = std::collections::HashSet::new();
                for p in params {
                    local_vars.insert(p.clone());
                }
                collect_local_stmt_vars(body, &mut local_vars);

                let mut body_idents = std::collections::HashSet::new();
                for s in body {
                    collect_stmt_identifiers(s, &mut body_idents);
                }
                for id in body_idents {
                    if !local_vars.contains(&id) {
                        function_idents.insert(id);
                    }
                }
            }
        }

        for stmt in &top_level {
            if let Stmt::Let {
                name,
                type_ann,
                value,
            } = stmt.inner_stmt()
            {
                if function_idents.contains(name) {
                    let symbol = format!("alya_global_{}", name);
                    let sname = if let Some(t) = type_ann {
                        let bare_base = t.split('[').next().unwrap_or(t);
                        let bare = bare_base.rsplit("::").next().unwrap_or(bare_base);
                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                        if self.ctx.structs.contains_key(bare_base)
                            || self.ctx.enums.contains(bare_base)
                        {
                            Some(bare_base.to_string())
                        } else if self.ctx.structs.contains_key(bare)
                            || self.ctx.enums.contains(bare)
                        {
                            Some(bare.to_string())
                        } else {
                            None
                        }
                    } else {
                        self.get_expr_struct_name(value)
                    };
                    self.ctx.globals.insert(name.clone(), (symbol, sname));
                }
            }
        }

        let t_emit = std::time::Instant::now();
        arch::emit_header(&mut self.output, self.arch, self.os);

        // B3: install the crash handler first, before any user code runs.
        if matches!(self.arch, Architecture::ARM64) {
            self.output.push_str("    bl alya_crash_init\n");
        } else if matches!(self.os, OperatingSystem::Windows) {
            self.output.push_str("    sub $32, %rsp\n");
            self.output.push_str("    call alya_crash_init\n");
            self.output.push_str("    add $32, %rsp\n");
        } else {
            self.output.push_str("    call alya_crash_init\n");
        }

        if self.mem_trace {
            if matches!(self.arch, Architecture::ARM64) {
                arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x0",
                    "alya_mem_trace_enabled",
                    self.os,
                );
                self.output.push_str("    mov x1, #1\n");
                self.output.push_str("    str x1, [x0]\n");
                arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x0",
                    "alya_mem_trace_report",
                    self.os,
                );
                if matches!(self.os, OperatingSystem::MacOS) {
                    self.output.push_str("    bl _atexit\n");
                } else {
                    self.output.push_str("    bl atexit\n");
                }
            } else {
                // x64
                self.output
                    .push_str("    movq $1, alya_mem_trace_enabled(%rip)\n");
                if matches!(self.os, OperatingSystem::Windows) {
                    self.output
                        .push_str("    lea alya_mem_trace_report(%rip), %rcx\n");
                    self.output.push_str("    sub $32, %rsp\n");
                    self.output.push_str("    call atexit\n");
                    self.output.push_str("    add $32, %rsp\n");
                } else if matches!(self.os, OperatingSystem::MacOS) {
                    self.output
                        .push_str("    lea alya_mem_trace_report(%rip), %rdi\n");
                    self.output.push_str("    call _atexit\n");
                } else {
                    self.output
                        .push_str("    lea alya_mem_trace_report(%rip), %rdi\n");
                    self.output.push_str("    call atexit\n");
                }
            }
        }

        if !self.ctx.globals.is_empty() {
            self.emit_data_section();
            let word_dir = ".quad";
            for (symbol, _) in self.ctx.globals.values() {
                let mangled = crate::codegen::arch::control::mangle_symbol_name(symbol);
                self.output.push_str(&format!(
                    ".global {}\n{}:\n    {} 0\n",
                    mangled, mangled, word_dir
                ));
            }
            self.output.push_str(".text\n");
        }

        self.emit_rodata_section();
        self.output.push_str("alya_rodata_guard:\n");
        self.output.push_str("    .space 64\n");
        self.output.push_str(".global alya_rodata_start\n");
        self.output.push_str("alya_rodata_start:\n");
        self.output.push_str("    .byte 0\n");
        self.output.push_str(".text\n");

        // Emit external symbol declarations
        let mut declared_externs = std::collections::HashSet::new();
        for name in self.ctx.extern_functions.keys() {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if declared_externs.insert(bare.to_string()) {
                if matches!(self.os, OperatingSystem::MacOS) {
                    self.output.push_str(&format!(".extern _{}\n", bare));
                } else {
                    self.output.push_str(&format!(".extern {}\n", bare));
                }
            }
        }

        // Functions generate before the top-level flow (alya-lang/alya#55-C):
        // every marker is installed before any caller — including
        // top-level — is generated, so only recursion still misses.
        // Generation order is not file order: bodies go to a side
        // buffer appended after the entry footer below, keeping the
        // file layout (entry flow falls through to its own `ret`)
        // exactly as before. enter/exit_function fully save and
        // restore scope state, so the top-level flow sees identical
        // context either way.
        // `@cold` functions emit last so hot code stays contiguous
        // (Chapter 18 §1.2). Order is otherwise callee-first; calls
        // resolve by name and the C entry point is fixed.
        let entry_output = std::mem::take(&mut self.output);
        let (hot, cold): (Vec<_>, Vec<_>) = functions.into_iter().partition(|func| {
            !matches!(func, Stmt::Function { attributes, .. } if attributes.iter().any(|a| a.name == "cold"))
        });
        for func in hot.into_iter().chain(cold) {
            if let Stmt::Function {
                name,
                params,
                param_types,
                body,
                attributes,
                ..
            } = func
            {
                self.generate_function(
                    name,
                    params,
                    param_types,
                    body,
                    attributes,
                    program,
                    &inference,
                );
            }
        }
        let func_output = std::mem::take(&mut self.output);
        self.output = entry_output;

        // Nullable-heap proof for the top-level flow (same as functions).
        let global_names: std::collections::HashSet<String> =
            self.ctx.globals.keys().cloned().collect();
        let proven = crate::codegen::analysis::nullable_heap_locals(
            &top_level,
            &[],
            &global_names,
            &self.ctx.variables,
            &self.ctx.structs,
        );
        self.ctx.nullable_heap_vars = proven;
        for stmt in top_level {
            self.generate_statement(stmt);
        }

        self.emit_cleanup_scope(None);

        if self.mem_trace {
            if matches!(self.arch, Architecture::ARM64) {
                self.output.push_str("    bl alya_mem_trace_report\n");
            } else if matches!(self.os, OperatingSystem::Windows) {
                self.output.push_str("    push %rbp\n");
                self.output.push_str("    mov %rsp, %rbp\n");
                self.output.push_str("    and $-16, %rsp\n");
                self.output.push_str("    sub $32, %rsp\n");
                self.output.push_str("    call alya_mem_trace_report\n");
                self.output.push_str("    mov %rbp, %rsp\n");
                self.output.push_str("    pop %rbp\n");
            } else {
                self.output.push_str("    push %rbp\n");
                self.output.push_str("    mov %rsp, %rbp\n");
                self.output.push_str("    and $-16, %rsp\n");
                self.output.push_str("    call alya_mem_trace_report\n");
                self.output.push_str("    mov %rbp, %rsp\n");
                self.output.push_str("    pop %rbp\n");
            }
        }

        if !self.no_std {
            match self.arch {
                Architecture::ARM64 => self.output.push_str("    bl fn___native_fiber_drain\n"),
                Architecture::X64 => {
                    if matches!(self.os, OperatingSystem::Windows) {
                        self.output.push_str("    sub $32, %rsp\n");
                        self.output.push_str("    call fn___native_fiber_drain\n");
                        self.output.push_str("    add $32, %rsp\n");
                    } else {
                        self.output.push_str("    call fn___native_fiber_drain\n");
                    }
                }
            }
        }

        arch::emit_footer(&mut self.output, self.arch);
        // Close the entry `main:` SEH scope (alya-lang/alya#109). The
        // footer ends with `ret`, so this is main's final position.
        arch::emit_function_endproc(&mut self.output, self.arch, self.os);

        // Function bodies generated up front (see above); appended here
        // so the file layout is unchanged: entry flow, its `ret`, then
        // labeled function blocks, then the runtime.
        self.output.push_str(&func_output);

        if !self.no_std {
            runtime::emit_runtime(
                &mut self.output,
                self.arch,
                self.os,
                &self.ctx.structs,
                &self.ctx.interfaces,
                &self.ctx.vtables,
                &self.ctx.functions,
                self.mem_trace,
            );
        }
        let d_codegen = t_emit.elapsed();

        // Return-tag protocol measurement (alya-lang/alya#55-C): report
        // supply (qualifying functions), demand hits, and misses split
        // into forward-reference (callee marked later — a pre-pass would
        // recover these) vs structural (never marked: recursion,
        // dynamics, disqualified bodies). Gated by `ALYA_TAG_STATS` so
        // normal builds stay byte-identical on stderr.
        if self.ctx.tag_stats.enabled {
            let stats = &self.ctx.tag_stats;
            let mut forward = 0u64;
            for miss in &stats.miss_names {
                let bare = miss.rsplit("::").next().unwrap_or(miss);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if self
                    .ctx
                    .variables
                    .contains_key(&format!("fn_ret_tagged:{}", miss))
                    || self
                        .ctx
                        .variables
                        .contains_key(&format!("fn_ret_tagged:{}", bare))
                {
                    forward += 1;
                }
            }
            let structural = stats.miss_names.len() as u64 - forward;
            eprintln!(
                "[tag-stats] markers={} call_hits={} misses={} (forward={} structural={} top_level_miss={})",
                stats.markers,
                stats.call_hits,
                stats.miss_names.len(),
                forward,
                structural,
                stats.miss_top_level
            );
        }

        self.profile = PipelineProfile {
            d_call_index,
            d_dce,
            d_inference,
            d_codegen,
            original_stmts,
            pruned_stmts,
        };
    }

    /// Return-tag protocol qualification (Phase 2b, alya-lang/alya#39):
    /// a function qualifies when every `return` leaves `(value, tag)` —
    /// a tag-carrying expression or an int/string/float/null literal
    /// (literals materialize at return; null materializes KIND_UNKNOWN)
    /// — AND every path returns (a fallthrough exit would leave a
    /// stale tag). Nested function bodies own their returns and are
    /// skipped. Anything else disqualifies (conservative: callers keep
    /// legacy behavior).
    fn fn_returns_all_tagged(
        body: &[Stmt],
        vars: &std::collections::HashMap<String, VarType>,
    ) -> bool {
        fn returns_ok(stmts: &[Stmt], vars: &std::collections::HashMap<String, VarType>) -> bool {
            for s in stmts {
                match s.inner_stmt() {
                    Stmt::Return(None) => return false,
                    Stmt::Return(Some(e)) => {
                        if !(analysis::is_tag_carrying_read(e, vars)
                            || matches!(
                                e,
                                &Expr::Number(_) | &Expr::String(_) | &Expr::Float(_) | &Expr::Null
                            ))
                        {
                            return false;
                        }
                    }
                    Stmt::Function { .. } => {}
                    Stmt::If {
                        then_block,
                        else_block,
                        ..
                    } => {
                        if !returns_ok(then_block, vars) {
                            return false;
                        }
                        if let Some(eb) = else_block {
                            if !returns_ok(eb, vars) {
                                return false;
                            }
                        }
                    }
                    Stmt::While { body, .. }
                    | Stmt::Repeat { body }
                    | Stmt::For { body, .. }
                    | Stmt::ForEach { body, .. } => {
                        if !returns_ok(body, vars) {
                            return false;
                        }
                    }
                    Stmt::TryCatch {
                        try_block,
                        catch_block,
                        finally_block,
                        ..
                    } => {
                        if !returns_ok(try_block, vars)
                            || !returns_ok(catch_block, vars)
                            || finally_block
                                .as_ref()
                                .is_some_and(|fb| !returns_ok(fb, vars))
                        {
                            return false;
                        }
                    }
                    Stmt::Defer(inner) | Stmt::Pub(inner)
                        if !returns_ok(std::slice::from_ref(inner), vars) =>
                    {
                        return false;
                    }
                    _ => {}
                }
            }
            true
        }
        /// True when a block cannot fall through: its last statement
        /// returns, throws, or nests such blocks on every path. Loops
        /// may exit and bare expressions fall through (conservative).
        fn block_diverges_or_returns(stmts: &[Stmt]) -> bool {
            let Some(last) = stmts.last() else {
                return false;
            };
            match last.inner_stmt() {
                Stmt::Return(_) | Stmt::Throw(_) => true,
                Stmt::If {
                    then_block,
                    else_block: Some(eb),
                    ..
                } => block_diverges_or_returns(then_block) && block_diverges_or_returns(eb),
                Stmt::If { .. } => false,
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    block_diverges_or_returns(try_block)
                        && block_diverges_or_returns(catch_block)
                        && finally_block
                            .as_ref()
                            .map_or(true, |fb| block_diverges_or_returns(fb))
                }
                Stmt::Defer(inner) | Stmt::Pub(inner) => {
                    block_diverges_or_returns(std::slice::from_ref(inner))
                }
                _ => false,
            }
        }
        returns_ok(body, vars) && block_diverges_or_returns(body)
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_function(
        &mut self,
        name: &str,
        params: &[String],
        param_types: &[Option<String>],
        body: &[Stmt],
        attributes: &[Attribute],
        program: &Program,
        inference: &ProgramInference,
    ) {
        let mut saved = self.ctx.enter_function();
        self.ctx.current_fn_name = name.to_string();

        // Nullable-heap proof for the probe-free fast path (see
        // `nullable_heap_locals`): which locals can only ever hold
        // `null` or heap values in this body.
        let global_names: std::collections::HashSet<String> =
            self.ctx.globals.keys().cloned().collect();
        let body_refs: Vec<&Stmt> = body.iter().collect();
        let proven = crate::codegen::analysis::nullable_heap_locals(
            &body_refs,
            params,
            &global_names,
            &self.ctx.variables,
            &self.ctx.structs,
        );
        self.ctx.nullable_heap_vars = proven;
        // Non-escaping heap params (see `nonescaping_params`): borrowed
        // for the whole call, so the entry retain and the scope release
        // are skipped as a balanced pair.
        self.ctx.nonescaping_params =
            crate::codegen::analysis::nonescaping_params(&body_refs, params, &global_names);

        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let prefix1 = format!("fn_local_str_arr:{}:", name);
        let prefix2 = format!("fn_local_str_arr:{}:", bare);
        for s in &inference.known_strings {
            if let Some(var_name) = s.strip_prefix(&prefix1) {
                self.ctx
                    .variables
                    .insert(format!("arr_is_str:{}", var_name), VarType::Number(0));
            } else if let Some(var_name) = s.strip_prefix(&prefix2) {
                self.ctx
                    .variables
                    .insert(format!("arr_is_str:{}", var_name), VarType::Number(0));
            }
        }

        // `@export("alias")`: global alias labels sharing the entry address,
        // so native consumers can link against the unmangled name. Emitted
        // BEFORE the prologue: both labels land on the same address.
        for attr in attributes {
            if attr.name == "export" {
                if let Some((_, alias)) = attr.args.first() {
                    if !alias.is_empty() {
                        arch::emit_export_alias(&mut self.output, self.arch, alias);
                    }
                }
            }
        }

        arch::emit_function_prologue(&mut self.output, self.arch, self.os, name);

        let mut heap_param_offsets = Vec::new();
        // Static call-site specialization (alya-lang/alya#39 Phase 2):
        // `{fn}__spk__{codes}` clones carry proven per-param kinds.
        // They override inference exactly like explicit annotations
        // (same value types, no new TypeErrors: checking still sees Any).
        let spec_codes: Vec<char> = crate::parser::dynspec::dynspec_codes(name).unwrap_or_default();
        let mut spec_seen = 0usize;
        for (i, param) in params.iter().enumerate() {
            // Explicit scalar annotations are authoritative and override
            // heuristic inference. Rationale: whole-program inference keys
            // string-ness by bare field name (`struct_field_str:port`), so an
            // unrelated struct's `port: string` field once poisoned an
            // explicitly-typed `port: int` parameter into string treatment
            // (`is null` compiled to strcmp, `say` segfaulted). An explicit
            // scalar annotation always wins; inference only applies to
            // unannotated (`any`) parameters.
            let explicit_scalar: Option<&str> =
                param_types.get(i).and_then(|t| t.as_deref()).and_then(|t| {
                    // Array/map/generic spellings (`string[]`, `map[...]`)
                    // are not scalars: the existing array/map logic below
                    // (plus inference) keeps applying to them.
                    if t.contains('[') || t.contains(']') {
                        return None;
                    }
                    let base = t.rsplit("::").next().unwrap_or(t);
                    let base = base.rsplit("__").next().unwrap_or(base);
                    match base {
                        "string" | "str" | "int" | "i64" | "isize" | "uint" | "u64" | "usize"
                        | "i32" | "i16" | "i8" | "u32" | "u16" | "u8" | "byte" | "float"
                        | "f64" | "f32" | "bool" | "boolean" | "rune" | "char" => Some(base),
                        _ => None,
                    }
                });
            let mut is_str = inference.infer_param_is_string(name, i, program);
            let mut is_flt = inference.infer_param_is_float(name, i, program);
            // Explicit collection annotations are enforced by the type
            // checker (mismatched arguments/lets are compile errors), so
            // they are exact — unlike the may-fact inference below.
            let annot_arr = param_types
                .get(i)
                .and_then(|t| t.as_deref())
                .is_some_and(|t| {
                    t == "..." || t.starts_with("...") || t == "array" || t.ends_with("[]")
                });
            let annot_map = param_types
                .get(i)
                .and_then(|t| t.as_deref())
                .is_some_and(|t| {
                    t == "map"
                        || t.starts_with("map[")
                        || (t.starts_with('[') && t.contains(':') && t.ends_with(']'))
                });
            // Explicit string annotations are enforced by the type
            // checker, so they are exact like the collection ones.
            let annot_str = param_types
                .get(i)
                .and_then(|t| t.as_deref())
                .is_some_and(|t| t == "string" || t == "str");
            let mut is_arr = inference.infer_param_is_array(name, i, program) || annot_arr;
            let mut is_str_arr = inference.infer_param_is_string_array(name, i, program);
            let mut is_flt_arr = inference.infer_param_is_float_array(name, i, program);
            // Explicit integer-array annotations are enforced by the
            // type checker, so they are exact element-kind facts.
            let is_int_arr = param_types
                .get(i)
                .and_then(|t| t.as_deref())
                .is_some_and(crate::codegen::analysis::is_int_array_annotation);
            let mut is_map = inference.infer_param_is_map(name, i, program) || annot_map;
            if let Some(s) = explicit_scalar {
                match s {
                    "string" | "str" => {
                        is_str = true;
                        is_flt = false;
                        is_arr = false;
                        is_str_arr = false;
                        is_flt_arr = false;
                        is_map = false;
                    }
                    "float" | "f64" | "f32" => {
                        is_str = false;
                        is_flt = true;
                        is_arr = false;
                        is_str_arr = false;
                        is_flt_arr = false;
                        is_map = false;
                    }
                    _ => {
                        is_str = false;
                        is_flt = false;
                        is_arr = false;
                        is_str_arr = false;
                        is_flt_arr = false;
                        is_map = false;
                    }
                }
            }
            // Specialized kind wins over inference (never over an
            // explicit annotation: only untyped params are specialized).
            // Struct/interface-typed values keep their own path below.
            let spec_kind: Option<char> =
                if param_types.get(i).and_then(|t| t.as_deref()).is_none() && param != "self" {
                    let code = spec_codes.get(spec_seen).copied();
                    spec_seen += 1;
                    code
                } else {
                    None
                };
            if let Some(kind) = spec_kind {
                is_str = kind == 's';
                is_flt = kind == 'f';
                is_arr = kind == 'a';
                is_str_arr = false;
                is_flt_arr = false;
                is_map = kind == 'm';
            }
            // Strict (must-) facts for `is` folding (alya-lang/alya#70):
            // the may-facts above stay authoritative for retains/heap
            // marking, but folding needs every caller proven. Enforced
            // annotations and dynspec suffix codes are exact by contract.
            let is_arr_strict = annot_arr
                || spec_kind == Some('a')
                || inference.infer_param_is_array_strict(name, i, program);
            let is_map_strict = annot_map
                || spec_kind == Some('m')
                || inference.infer_param_is_map_strict(name, i, program);
            // Strict string fact for `is string` folding
            // (alya-lang/alya#71): the may-fact also covers
            // merely-possible strings, so folding trusts only enforced
            // annotations and dynspec codes here.
            let is_str_strict = annot_str || spec_kind == Some('s');
            let struct_type = if let Some(Some(t)) = param_types.get(i) {
                let bare_base = t.split('[').next().unwrap_or(t);
                let b = bare_base.rsplit("::").next().unwrap_or(bare_base);
                let b = b.rsplit("__").next().unwrap_or(b);
                if self.ctx.structs.contains_key(bare_base) {
                    Some(bare_base.to_string())
                } else if self.ctx.structs.contains_key(b) {
                    Some(b.to_string())
                } else {
                    None
                }
            } else {
                let method_receiver = if i == 0 && param == "self" {
                    let bare = name.rsplit("::").next().unwrap_or(name);
                    let parts: Vec<&str> = bare.split("__").collect();
                    if parts.len() >= 2 {
                        let mut found = None;
                        for part in &parts[..parts.len() - 1] {
                            if self.ctx.structs.contains_key(*part) {
                                found = Some(part.to_string());
                                break;
                            }
                            for sname in self.ctx.structs.keys() {
                                let s_bare = sname.rsplit("::").next().unwrap_or(sname);
                                let s_bare = s_bare.rsplit("__").next().unwrap_or(s_bare);
                                if s_bare == *part {
                                    found = Some(sname.clone());
                                    break;
                                }
                            }
                        }
                        found
                    } else {
                        None
                    }
                } else {
                    None
                };

                method_receiver.or_else(|| inference.infer_param_struct_type(name, i))
            };
            let interface_type = if let Some(Some(t)) = param_types.get(i) {
                let bare_base = t.split('[').next().unwrap_or(t);
                let b = bare_base.rsplit("::").next().unwrap_or(bare_base);
                let b = b.rsplit("__").next().unwrap_or(b);
                if self.ctx.interfaces.contains_key(bare_base) {
                    Some(bare_base.to_string())
                } else if self.ctx.interfaces.contains_key(b) {
                    Some(b.to_string())
                } else {
                    None
                }
            } else {
                None
            };
            arch::emit_function_param_push(
                &mut self.output,
                self.arch,
                i,
                &mut self.ctx.stack_offset,
                self.os,
            );
            if let Some(ref sname) = struct_type {
                self.ctx.variables.insert(
                    param.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: self.ctx.stack_offset,
                    },
                );
            } else if let Some(ref iname) = interface_type {
                self.ctx.variables.insert(
                    param.clone(),
                    VarType::Interface {
                        interface_name: iname.clone(),
                        offset: self.ctx.stack_offset,
                    },
                );
            } else if is_str {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::StringOffset(self.ctx.stack_offset));
                if is_str_strict {
                    self.ctx
                        .variables
                        .insert(format!("param_str_strict:{}", param), VarType::Number(0));
                }
            } else if is_flt {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Float(self.ctx.stack_offset));
            } else if is_arr || is_str_arr || is_flt_arr {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Array(self.ctx.stack_offset));
                if is_arr_strict {
                    self.ctx
                        .variables
                        .insert(format!("param_arr_strict:{}", param), VarType::Number(0));
                }
                if is_str_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_str:{}", param), VarType::Number(0));
                }
                if is_flt_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_flt:{}", param), VarType::Number(0));
                    // Annotated float-array params convert on read (#95).
                    let ann_key1 = format!("fn_param_flt_arr_ann:{}:{}", name, i);
                    let ann_key2 = format!("fn_param_flt_arr_ann:{}:{}", bare, i);
                    // #101: qualified-spelled functions consult exact only.
                    let ann_hit = inference.known_floats.contains(&ann_key1)
                        || (crate::codegen::analysis::is_simple_name(name)
                            && inference.known_floats.contains(&ann_key2));
                    if ann_hit {
                        self.ctx
                            .variables
                            .insert(format!("arr_flt_ann:{}", param), VarType::Number(0));
                    }
                }
                if is_int_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_int:{}", param), VarType::Number(0));
                }
            } else if is_map {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Map(self.ctx.stack_offset));
                if is_map_strict {
                    self.ctx
                        .variables
                        .insert(format!("param_map_strict:{}", param), VarType::Number(0));
                }
            } else {
                if param_types.get(i).and_then(|t| t.as_deref()).is_none() {
                    // Specialized params skip the untyped marker (their
                    // kind is proven); int-specialized ones gain the int
                    // marker exactly like explicitly-typed ints.
                    if spec_kind.is_none() {
                        self.ctx
                            .variables
                            .insert(format!("param_is_untyped:{}", param), VarType::Number(0));
                    } else if spec_kind == Some('i') {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", param), VarType::Number(0));
                    }
                } else if let Some(Some(t)) = param_types.get(i) {
                    if matches!(
                        t.as_str(),
                        "int" | "i64" | "i32" | "u64" | "u32" | "uint" | "byte" | "char" | "bool"
                    ) {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", param), VarType::Number(0));
                    }
                    // Unsigned marker for u64-family annotations (B4).
                    if matches!(t.as_str(), "u64" | "u32" | "uint" | "usize") {
                        self.ctx
                            .variables
                            .insert(format!("var_is_uint:{}", param), VarType::Number(0));
                    }
                }
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Number(self.ctx.stack_offset));
            }

            let is_heap_param = struct_type.is_some()
                || interface_type.is_some()
                || is_arr
                || is_str_arr
                || is_flt_arr
                || is_map;
            if is_heap_param {
                heap_param_offsets.push((param.clone(), self.ctx.stack_offset));
            }
        }

        for (pname, offset) in heap_param_offsets {
            // Non-escaping params are borrowed for the whole call (see
            // `nonescaping_params`): skip the entry retain; the scope
            // release is skipped to match (see
            // `get_scope_heap_offsets`). All other params keep the
            // probed retain — the caller's actual value is invisible.
            if self.ctx.nonescaping_params.contains(&pname) {
                continue;
            }
            arch::emit_load_var(&mut self.output, self.arch, offset, self.ctx.stack_offset);
            arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }

        // Return-tag protocol (Phase 2b, alya-lang/alya#39): when every
        // return leaves (value, tag), record it so later callers can
        // dispatch on the tag. The marker must outlive this function,
        // so it goes into the saved (outer) scope as well —
        // exit_function drops the current one. Emission is
        // callee-first (alya-lang/alya#55-C), so only recursion still
        // misses the marker and keeps legacy behavior.
        if Self::fn_returns_all_tagged(body, &self.ctx.variables) {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            // Measurement for alya-lang/alya#55-C (supply side).
            // Gated: disabled builds skip even the counter.
            if self.ctx.tag_stats.enabled {
                self.ctx.tag_stats.markers += 1;
            }
            // #101: bare markers need a single owner (ambiguous
            // bares stay unrecorded; qualified-exact markers stay
            // precise and qualified-spelled calls never consult bare).
            let mut keys = vec![format!("fn_ret_tagged:{}", name)];
            if crate::codegen::analysis::may_record_bare_vars(&self.ctx.variables, name, bare) {
                keys.push(format!("fn_ret_tagged:{}", bare));
            }
            for key in keys {
                self.ctx.variables.insert(key.clone(), VarType::Number(0));
                saved.variables.insert(key, VarType::Number(0));
            }
        }

        let defers = collect_all_defers(body);
        if !defers.is_empty() {
            let mut defer_entries = Vec::new();
            for (idx, def_stmt) in defers.into_iter().enumerate() {
                arch::emit_load_num(&mut self.output, self.arch, 0);
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                let offset = self.ctx.stack_offset;
                defer_entries.push((idx, def_stmt, offset));
            }
            self.ctx.active_defers = defer_entries;
        }

        for stmt in body {
            self.generate_statement(stmt);
        }

        self.emit_run_defers();
        self.emit_cleanup_scope(None);

        // Return-tag protocol: a fallthrough (non-return) exit leaves
        // unknown tag so callers never trust a stale one. Reachable only
        // when qualification missed a path; defense in depth.
        {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if self
                .ctx
                .variables
                .contains_key(&format!("fn_ret_tagged:{}", name))
                || self
                    .ctx
                    .variables
                    .contains_key(&format!("fn_ret_tagged:{}", bare))
            {
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movl $0, %edx\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    mov w1, #0\n");
                    }
                }
            }
        }

        arch::emit_function_epilogue(&mut self.output, self.arch);
        // Windows-x64 SEH scope close (alya-lang/alya#109): once per
        // function at its final `.text` position. Code emission always
        // ends in `.text` (every rodata hop switches back), and every
        // `return` inlines its own epilogue above, so this is the single
        // function end.
        arch::emit_function_endproc(&mut self.output, self.arch, self.os);

        self.ctx.exit_function(saved);
    }

    pub(crate) fn get_scope_heap_offsets(&self, skip_offset: Option<i32>) -> Vec<i32> {
        let mut offsets: Vec<i32> = self
            .ctx
            .variables
            .iter()
            .filter(|(name, _)| !name.contains(':') && !name.contains('.'))
            // Non-escaping params hold a borrow, not a share: no entry
            // retain was emitted, so no scope release may be either.
            .filter(|(name, _)| !self.ctx.nonescaping_params.contains(*name))
            .filter_map(|(_, vtype)| match vtype {
                VarType::Array(off)
                | VarType::Map(off)
                | VarType::Struct { offset: off, .. }
                | VarType::Interface { offset: off, .. } => {
                    if *off > 0 && Some(*off) != skip_offset {
                        Some(*off)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        offsets.sort_unstable();
        offsets.dedup();
        offsets
    }

    /// Provenness for a scope-cleanup slot: direct release only when
    /// every heap-typed name currently mapped to `offset` is
    /// union-proven. The union (every write to the name is null-or-heap)
    /// covers the current value exactly; stale typed slots keep the
    /// probed call. Offsets shared by several names (slot reuse) take
    /// the probe unless all are proven.
    pub(crate) fn scope_heap_slot_proven(&self, offset: i32) -> bool {
        let mut found = false;
        for (name, vtype) in &self.ctx.variables {
            if name.contains(':') || name.contains('.') {
                continue;
            }
            let off = match vtype {
                VarType::Array(o) | VarType::Map(o) => *o,
                VarType::Struct { offset: o, .. } | VarType::Interface { offset: o, .. } => *o,
                _ => continue,
            };
            if off != offset {
                continue;
            }
            found = true;
            if !self.ctx.nullable_heap_vars.contains(name) {
                return false;
            }
        }
        found
    }

    pub(crate) fn emit_cleanup_scope(&mut self, skip_offset: Option<i32>) {
        let offsets = self.get_scope_heap_offsets(skip_offset);
        for offset in offsets {
            // Direct only for union-proven slots (see
            // `scope_heap_slot_proven`); everything else keeps the probe.
            if self.scope_heap_slot_proven(offset) {
                arch::emit_rc_release_stack_direct(
                    &mut self.output,
                    self.arch,
                    offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                arch::emit_rc_release_stack(
                    &mut self.output,
                    self.arch,
                    offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            }
        }
    }

    pub(crate) fn emit_rc_release_scope_return(
        &mut self,
        heap_offsets: &[i32],
        check_return_match: bool,
    ) {
        if !check_return_match || heap_offsets.is_empty() {
            for &offset in heap_offsets {
                // Direct only for union-proven slots (see
                // `scope_heap_slot_proven`).
                if self.scope_heap_slot_proven(offset) {
                    arch::emit_rc_release_stack_direct(
                        &mut self.output,
                        self.arch,
                        offset,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else {
                    arch::emit_rc_release_stack(
                        &mut self.output,
                        self.arch,
                        offset,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
            }
            return;
        }

        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    pushq $0\n");
                self.ctx.stack_offset += 8;
                for &offset in heap_offsets {
                    let uid = self.ctx.next_label();
                    let clean_uid = uid.trim_start_matches('.');
                    self.output.push_str("    cmpq $0, (%rsp)\n");
                    self.output
                        .push_str(&format!("    jne .L_rel_{}\n", clean_uid));
                    self.output
                        .push_str(&format!("    mov -{}(%rbp), %rax\n", offset));
                    self.output.push_str("    cmp 8(%rsp), %rax\n");
                    self.output
                        .push_str(&format!("    jne .L_rel_{}\n", clean_uid));
                    self.output.push_str("    test %rax, %rax\n");
                    self.output
                        .push_str(&format!("    jz .L_rel_{}\n", clean_uid));
                    self.output.push_str("    movq $1, (%rsp)\n");
                    self.output
                        .push_str(&format!("    jmp .L_skip_{}\n", clean_uid));
                    self.output.push_str(&format!(".L_rel_{}:\n", clean_uid));
                    // Direct only for union-proven slots (see
                    // `scope_heap_slot_proven`).
                    if self.scope_heap_slot_proven(offset) {
                        arch::emit_rc_release_stack_direct(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else {
                        arch::emit_rc_release_stack(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    self.output.push_str(&format!(".L_skip_{}:\n", clean_uid));
                }
                self.output.push_str("    pop %r11\n");
                self.ctx.stack_offset -= 8;
            }
            Architecture::ARM64 => {
                self.output.push_str("    str xzr, [sp, #8]\n");
                for &offset in heap_offsets {
                    let uid = self.ctx.next_label();
                    let clean_uid = uid.trim_start_matches('.');
                    self.output.push_str("    ldr x9, [sp, #8]\n");
                    self.output
                        .push_str(&format!("    cbnz x9, .L_rel_{}\n", clean_uid));
                    arch::arm64::emit_arm64_load_x29_offset(&mut self.output, "x1", offset, "x9");
                    self.output.push_str("    ldr x2, [sp]\n");
                    self.output.push_str("    cmp x1, x2\n");
                    self.output
                        .push_str(&format!("    bne .L_rel_{}\n", clean_uid));
                    self.output
                        .push_str(&format!("    cbz x1, .L_rel_{}\n", clean_uid));
                    self.output.push_str("    mov x9, #1\n");
                    self.output.push_str("    str x9, [sp, #8]\n");
                    self.output
                        .push_str(&format!("    b .L_skip_{}\n", clean_uid));
                    self.output.push_str(&format!(".L_rel_{}:\n", clean_uid));
                    // Direct only for union-proven slots (see
                    // `scope_heap_slot_proven`).
                    if self.scope_heap_slot_proven(offset) {
                        arch::emit_rc_release_stack_direct(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else {
                        arch::emit_rc_release_stack(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    self.output.push_str(&format!(".L_skip_{}:\n", clean_uid));
                }
            }
        }
    }

    /// True when a user-defined `assert`/`assert_eq` could serve a builtin-form
    /// call with `nargs` arguments, in which case builtin desugaring must NOT
    /// shadow it. Plain/bare names match by arity; `Struct__name` method
    /// entries match only when the first argument's struct agrees, so an
    /// unrelated struct's method never hijacks a builtin call.
    pub(crate) fn user_assert_shadows(&self, base: &str, args: &[crate::ast::Expr]) -> bool {
        let nargs = args.len();
        let fits = |req: usize, total: usize| req <= nargs && nargs <= total;
        for (key, (req, total)) in &self.ctx.fn_arities {
            let bare = key.rsplit("::").next().unwrap_or(key);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if key == base || bare == base {
                if fits(*req, *total) {
                    return true;
                }
                continue;
            }
            // Method entry (`Struct__assert_eq`): only counts with a matching
            // receiver struct as first argument.
            let is_method_shape =
                key.ends_with(&format!("__{}", base)) || key.ends_with(&format!("::{}", base));
            if !is_method_shape {
                continue;
            }
            if !fits(*req, *total) {
                continue;
            }
            if let Some(first) = args.first() {
                if let Some(sname) = self.get_expr_struct_name(first) {
                    let struct_bare = sname.rsplit("::").next().unwrap_or(&sname);
                    let struct_bare = struct_bare.rsplit("__").next().unwrap_or(struct_bare);
                    let head = key
                        .strip_suffix(&format!("__{}", base))
                        .or_else(|| key.strip_suffix(&format!("::{}", base)))
                        .unwrap_or(key.as_str());
                    let head_bare = head.rsplit("::").next().unwrap_or(head);
                    let head_bare = head_bare.rsplit("__").next().unwrap_or(head_bare);
                    if head == sname || head_bare == struct_bare {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn get_expr_struct_name(&self, expr: &crate::ast::Expr) -> Option<String> {
        use crate::ast::Expr;
        match expr {
            Expr::Identifier(name) => {
                if let Some((_, Some(sname))) = self.ctx.globals.get(name) {
                    return Some(sname.clone());
                }
                if let Some(VarType::Struct { struct_name, .. }) = self.ctx.variables.get(name) {
                    Some(struct_name.clone())
                } else if let Some(VarType::StringLabel(ename)) =
                    self.ctx.variables.get(&format!("var_enum_type:{}", name))
                {
                    Some(ename.clone())
                } else {
                    let bare = name.rsplit("::").next().unwrap_or(name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if self.ctx.structs.contains_key(name) || self.ctx.enums.contains(name) {
                        Some(name.clone())
                    } else if self.ctx.structs.contains_key(bare) || self.ctx.enums.contains(bare) {
                        Some(bare.to_string())
                    } else if let Some(VarType::Struct { struct_name, .. }) = self
                        .ctx
                        .variables
                        .get(&format!("fn_ret_struct:{}", name))
                        .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", bare)))
                    {
                        Some(struct_name.clone())
                    } else if let Some(VarType::StringLabel(ename)) =
                        self.ctx.variables.get(&format!("var_enum_type:{}", bare))
                    {
                        Some(ename.clone())
                    } else {
                        None
                    }
                }
            }
            Expr::FieldAccess { object, field } | Expr::OptionalFieldAccess { object, field } => {
                // 1. Resolve parent struct recursively
                if let Some(parent_struct) = self.get_expr_struct_name(object) {
                    let bare_parent = parent_struct.rsplit("::").next().unwrap_or(&parent_struct);
                    let bare_parent = bare_parent.rsplit("__").next().unwrap_or(bare_parent);
                    if let Some(VarType::Struct { struct_name, .. }) = self
                        .ctx
                        .variables
                        .get(&format!("struct_field_struct:{}.{}", parent_struct, field))
                        .or_else(|| {
                            self.ctx
                                .variables
                                .get(&format!("struct_field_struct:{}.{}", bare_parent, field))
                        })
                    {
                        return Some(struct_name.clone());
                    }
                    if let Some(sdef) = self
                        .ctx
                        .structs
                        .get(&parent_struct)
                        .or_else(|| self.ctx.structs.get(bare_parent))
                    {
                        if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                            if let Some(Some(ftype)) = sdef.field_types.get(idx) {
                                let bare_ftype = ftype.rsplit("::").next().unwrap_or(ftype);
                                let bare_ftype =
                                    bare_ftype.rsplit("__").next().unwrap_or(bare_ftype);
                                if self.ctx.structs.contains_key(ftype) {
                                    return Some(ftype.clone());
                                } else if self.ctx.structs.contains_key(bare_ftype) {
                                    return Some(bare_ftype.to_string());
                                } else {
                                    return None;
                                }
                            }
                        }
                    }
                }
                // 2. Fallback to global field struct mapping
                if let Some(VarType::Struct { struct_name, .. }) = self
                    .ctx
                    .variables
                    .get(&format!("struct_field_struct:{}", field))
                {
                    Some(struct_name.clone())
                } else {
                    None
                }
            }
            Expr::Call { name, args } => {
                if name == "new" {
                    if let Some(first_arg) = args.first() {
                        let target_type = match first_arg {
                            Expr::Identifier(id) => Some(id.as_str()),
                            Expr::FieldAccess { field, .. } => Some(field.as_str()),
                            Expr::Index { array, .. } => match &**array {
                                Expr::Identifier(id) => Some(id.as_str()),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(tname) = target_type {
                            let bare = tname.rsplit("::").next().unwrap_or(tname);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            if self.ctx.structs.contains_key(tname) {
                                return Some(tname.to_string());
                            } else if self.ctx.structs.contains_key(bare) {
                                return Some(bare.to_string());
                            }
                        }
                    }
                }
                if let Some(target) = name
                    .strip_suffix("__new")
                    .or_else(|| name.strip_suffix("::new"))
                {
                    let bare = target.rsplit("::").next().unwrap_or(target);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if self.ctx.structs.contains_key(target) {
                        return Some(target.to_string());
                    }
                    if self.ctx.structs.contains_key(bare) {
                        return Some(bare.to_string());
                    }
                }
                if let Some(first_arg) = args.first() {
                    if let Some(st) = self.get_expr_struct_name(first_arg) {
                        let bare_st = st.rsplit("::").next().unwrap_or(&st);
                        let bare_st = bare_st.rsplit("__").next().unwrap_or(bare_st);
                        let candidate1 = format!("{}__{}", st, name);
                        let candidate2 = format!("{}__{}", bare_st, name);
                        if let Some(VarType::Struct { struct_name, .. }) = self
                            .ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", candidate1))
                            .or_else(|| {
                                self.ctx
                                    .variables
                                    .get(&format!("fn_ret_struct:{}", candidate2))
                            })
                        {
                            return Some(struct_name.clone());
                        }
                    }
                }
                let ns_bare = name.rsplit("::").next().unwrap_or(name);
                let bare = if let Some((prefix, _)) = ns_bare.split_once("__") {
                    if self.ctx.structs.contains_key(prefix) {
                        ns_bare
                    } else {
                        ns_bare.rsplit("__").next().unwrap_or(ns_bare)
                    }
                } else {
                    ns_bare
                };
                if self.ctx.structs.contains_key(name) {
                    Some(name.clone())
                } else if self.ctx.structs.contains_key(bare) {
                    Some(bare.to_string())
                } else if let Some(VarType::Struct { struct_name, .. }) = self
                    .ctx
                    .variables
                    .get(&format!("fn_ret_struct:{}", name))
                    .or_else(|| {
                        let mangled = name.replace("::", "__");
                        self.ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", mangled))
                    })
                    .or_else(|| {
                        let colon = name.replace("__", "::");
                        self.ctx.variables.get(&format!("fn_ret_struct:{}", colon))
                    })
                    .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", bare)))
                {
                    Some(struct_name.clone())
                } else {
                    None
                }
            }
            Expr::OptionalCall { callee, args } => {
                if let Some(first_arg) = args.first() {
                    if let Some(st) = self.get_expr_struct_name(first_arg) {
                        let bare_st = st.rsplit("::").next().unwrap_or(&st);
                        let bare_st = bare_st.rsplit("__").next().unwrap_or(bare_st);
                        let candidate1 = format!("{}__{}", st, callee);
                        let candidate2 = format!("{}__{}", bare_st, callee);
                        if let Some(VarType::Struct { struct_name, .. }) = self
                            .ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", candidate1))
                            .or_else(|| {
                                self.ctx
                                    .variables
                                    .get(&format!("fn_ret_struct:{}", candidate2))
                            })
                        {
                            return Some(struct_name.clone());
                        }
                    }
                }
                let ns_bare = callee.rsplit("::").next().unwrap_or(callee);
                let bare = if let Some((prefix, _)) = ns_bare.split_once("__") {
                    if self.ctx.structs.contains_key(prefix) {
                        ns_bare
                    } else {
                        ns_bare.rsplit("__").next().unwrap_or(ns_bare)
                    }
                } else {
                    ns_bare
                };
                if self.ctx.structs.contains_key(callee) {
                    Some(callee.clone())
                } else if self.ctx.structs.contains_key(bare) {
                    Some(bare.to_string())
                } else if let Some(VarType::Struct { struct_name, .. }) = self
                    .ctx
                    .variables
                    .get(&format!("fn_ret_struct:{}", callee))
                    .or_else(|| {
                        let mangled = callee.replace("::", "__");
                        self.ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", mangled))
                    })
                    .or_else(|| {
                        let colon = callee.replace("__", "::");
                        self.ctx.variables.get(&format!("fn_ret_struct:{}", colon))
                    })
                    .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", bare)))
                {
                    Some(struct_name.clone())
                } else {
                    None
                }
            }
            Expr::StructInit { name, .. } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if self.ctx.structs.contains_key(name) {
                    Some(name.clone())
                } else if self.ctx.structs.contains_key(bare) {
                    Some(bare.to_string())
                } else {
                    Some(name.clone())
                }
            }
            Expr::Ternary {
                then_branch,
                else_branch,
                ..
            } => self
                .get_expr_struct_name(then_branch)
                .or_else(|| self.get_expr_struct_name(else_branch)),
            Expr::NullCoalesce { value, default } => self
                .get_expr_struct_name(value)
                .or_else(|| self.get_expr_struct_name(default)),
            Expr::Binary { left, op, .. } => {
                if let Some(sname) = self.get_expr_struct_name(left) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let op_str = match op {
                        BinaryOp::Add => Some("+"),
                        BinaryOp::Subtract => Some("-"),
                        BinaryOp::Multiply => Some("*"),
                        BinaryOp::Divide => Some("/"),
                        BinaryOp::Modulo => Some("%"),
                        _ => None,
                    };
                    if let Some(op_sym) = op_str {
                        let cand1 = format!("{}__{}{}", sname, "operator", op_sym);
                        let cand2 = format!("{}__{}{}", bare_sname, "operator", op_sym);
                        if let Some(VarType::Struct { struct_name, .. }) = self
                            .ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", cand1))
                            .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", cand2)))
                        {
                            return Some(struct_name.clone());
                        }
                    }
                }
                None
            }
            Expr::Unary { op, expr } => {
                if *op == crate::ast::UnaryOp::Negate {
                    if let Some(sname) = self.get_expr_struct_name(expr) {
                        let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                        let cand1 = format!("{}__{}", sname, "operator-neg");
                        let cand2 = format!("{}__{}", bare_sname, "operator-neg");
                        if let Some(VarType::Struct { struct_name, .. }) = self
                            .ctx
                            .variables
                            .get(&format!("fn_ret_struct:{}", cand1))
                            .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", cand2)))
                        {
                            return Some(struct_name.clone());
                        }
                    }
                }
                None
            }
            Expr::Index { array, index } => {
                if let (Expr::Identifier(arr_name), Expr::Number(idx)) = (&**array, &**index) {
                    let key = format!("tuple_elem_struct:{}:{}", arr_name, idx);
                    if let Some(VarType::Struct { struct_name, .. }) = self.ctx.variables.get(&key)
                    {
                        return Some(struct_name.clone());
                    }
                }
                if let Some(sname) = self.get_expr_struct_name(array) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let cand1 = format!("{}__{}", sname, "operator[]");
                    let cand2 = format!("{}__{}", bare_sname, "operator[]");
                    if let Some(VarType::Struct { struct_name, .. }) = self
                        .ctx
                        .variables
                        .get(&format!("fn_ret_struct:{}", cand1))
                        .or_else(|| self.ctx.variables.get(&format!("fn_ret_struct:{}", cand2)))
                    {
                        return Some(struct_name.clone());
                    }
                }
                None
            }
            Expr::ForceUnwrap(inner) => self.get_expr_struct_name(inner),
            _ => None,
        }
    }

    pub(crate) fn get_expr_interface_name(&self, expr: &crate::ast::Expr) -> Option<String> {
        use crate::ast::Expr;
        match expr {
            Expr::Identifier(name) => {
                if let Some(VarType::Interface { interface_name, .. }) =
                    self.ctx.variables.get(name)
                {
                    Some(interface_name.clone())
                } else {
                    None
                }
            }
            Expr::ForceUnwrap(inner) => self.get_expr_interface_name(inner),
            _ => None,
        }
    }

    pub(crate) fn is_heap_expression(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr;
        match expr {
            Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } => true,
            Expr::Identifier(name) => matches!(
                self.ctx.variables.get(name),
                Some(VarType::Array(_))
                    | Some(VarType::Map(_))
                    | Some(VarType::Struct { .. })
                    | Some(VarType::Interface { .. })
            ),
            Expr::Call { name, .. } => {
                self.ctx.structs.contains_key(name)
                    || matches!(
                        self.ctx.variables.get(&format!("fn_ret_struct:{}", name)),
                        Some(VarType::Struct { .. })
                    )
                    || crate::codegen::analysis::is_array_expr(expr, &self.ctx.variables)
                    || crate::codegen::analysis::is_map_expr(expr, &self.ctx.variables)
            }
            Expr::FieldAccess { .. } | Expr::Index { .. } => {
                self.get_expr_struct_name(expr).is_some()
                    || crate::codegen::analysis::is_array_expr(expr, &self.ctx.variables)
                    || crate::codegen::analysis::is_map_expr(expr, &self.ctx.variables)
            }
            Expr::Ternary {
                then_branch,
                else_branch,
                ..
            } => self.is_heap_expression(then_branch) || self.is_heap_expression(else_branch),
            Expr::NullCoalesce { value, default } => {
                self.is_heap_expression(value) || self.is_heap_expression(default)
            }
            Expr::ForceUnwrap(inner) => self.is_heap_expression(inner),
            _ => false,
        }
    }

    /// Proven-heap for the probe-free direct retain: either a freshly
    /// created heap value (literal, heap-returning call — the value in
    /// the register IS the new object, so no stale-type window exists)
    /// or an identifier in `nullable_heap_vars` (every syntactic write
    /// to it is `null` or heap, so reads hold null — skipped by the
    /// guards — or live heap — magic check). Notably NOT typed locals:
    /// conditional writes can leave a stale heap type over a scalar
    /// value, and direct (prob-free) calls fault or corrupt on such
    /// values while probed calls skip them. Index/field reads are
    /// dynamic and never proven. Composite ternaries need both arms.
    pub(crate) fn value_proven_heap(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr as E;
        match expr {
            E::Array(_) | E::Map(_) | E::StructInit { .. } => true,
            E::Identifier(name) => self.ctx.nullable_heap_vars.contains(name),
            E::Ternary {
                then_branch,
                else_branch,
                ..
            } => self.value_proven_heap(then_branch) && self.value_proven_heap(else_branch),
            E::NullCoalesce { value, default } => {
                self.value_proven_heap(value) && self.value_proven_heap(default)
            }
            E::ForceUnwrap(inner) => self.value_proven_heap(inner),
            E::Call { .. } => self.is_heap_expression(expr),
            _ => false,
        }
    }

    /// Retain-needed test: pre-two-tier `is_heap_expression` plus
    /// union-proven untyped locals. Extra retains are the safe
    /// direction (at worst a leak); missing ones are use-after-free.
    pub(crate) fn value_needs_heap_retain(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr as E;
        if self.is_heap_expression(expr) {
            return true;
        }
        matches!(expr, E::Identifier(name) if self.ctx.nullable_heap_vars.contains(name))
    }

    /// Direct-release eligibility for a rebind of `name`: union proof
    /// only. The released old value was written by an earlier write to
    /// the same name, so the union (every write is null-or-heap)
    /// covers it exactly. Typed slots can go stale across conditional
    /// writes and must keep the probed call.
    pub(crate) fn slot_proven_heap(&self, name: &str) -> bool {
        self.ctx.nullable_heap_vars.contains(name)
    }

    /// Retain gate for collection stores (`push`, map `set`, array `set`, literal construction).
    /// Storing an aliased heap value must retain it so it survives later
    /// drops of the producer slot (loop-end releases, scope restores).
    /// But a proven scalar is never a heap pointer, and `rc_retain`
    /// faults on large 8-aligned ints (they pass its pointer guards and
    /// it reads `-16(ptr)`): skip the retain for proven-int/float
    /// identifiers and for reads from proven-int/float arrays. Anything
    /// else (notably dynamic map/index reads or tuple-destructured
    /// values) keeps the retain so it avoids use-after-free.
    pub(crate) fn store_value_needs_retain(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr;

        // Literals and arithmetic/comparison expressions are never heap objects.
        match expr {
            Expr::Number(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::InterpolatedString(_)
            | Expr::Null => return false,
            Expr::Binary { .. } | Expr::Unary { .. } => return false,
            _ => {}
        }

        if self.is_heap_expression(expr) {
            return true;
        }

        if crate::codegen::analysis::is_float_expr(expr, &self.ctx.variables)
            || crate::codegen::analysis::is_string_expr(expr, &self.ctx.variables)
        {
            return false;
        }

        match expr {
            Expr::Identifier(name) => {
                if self
                    .ctx
                    .variables
                    .contains_key(&format!("var_is_int:{}", name))
                {
                    return false;
                }
                // NOTE: a bare `Number` tracking type is NOT proof of an
                // int (e.g. tuple-destructured heap values are recorded
                // as Number): unknowns keep the retain. Only the
                // `var_is_int` marker above is an exact int fact.
                true
            }
            Expr::Index { array, .. } => {
                // If it's a map read (e.g. ni["bytes"]), the value might be a heap
                // object stored in the map (array, map, struct). We must retain it
                // so it survives when the enclosing map drops!
                if crate::codegen::analysis::is_map_expr(array, &self.ctx.variables) {
                    return true;
                }
                // Proven-scalar element reads are never heap pointers:
                // retaining a large 8-aligned int faults in rc_retain.
                if let Expr::Identifier(base) = array.as_ref() {
                    // arr_is_flt agrees with the tag only for proven
                    // float-only arrays (#95: arr_nonflt vetoes mixed).
                    if self
                        .ctx
                        .variables
                        .contains_key(&format!("arr_is_int:{}", base))
                        || (self
                            .ctx
                            .variables
                            .contains_key(&format!("arr_is_flt:{}", base))
                            && !self
                                .ctx
                                .variables
                                .contains_key(&format!("arr_nonflt:{}", base)))
                    {
                        return false;
                    }
                }
                if crate::codegen::analysis::is_float_expr(expr, &self.ctx.variables) {
                    return false;
                }
                // Anything else keeps the retain. In particular arrays of
                // statically-unknown element kind may hold heap objects
                // (e.g. parser piece lists of AstNode structs): skipping
                // the retain is a deterministic use-after-free once the
                // container drops, while retaining a small int is harmless
                // (rc_retain's pointer guards skip it).
                true
            }
            Expr::FieldAccess { object, field } => {
                if let Expr::Identifier(obj_name) = object.as_ref() {
                    if let Some(VarType::Struct { struct_name, .. }) =
                        self.ctx.variables.get(obj_name)
                    {
                        if let Some(sinfo) = self.ctx.structs.get(struct_name) {
                            if let Some(idx) = sinfo.fields.iter().position(|f| f == field) {
                                if let Some(Some(ftype)) = sinfo.field_types.get(idx) {
                                    if crate::codegen::analysis::is_int_scalar_annotation(ftype)
                                        || ftype == "float"
                                        || ftype == "f64"
                                        || ftype == "f32"
                                        || ftype == "string"
                                        || ftype == "str"
                                    {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Emits an RC retain guarded by the value-kind tag fresh in the tag
    /// register (x64 `%edx`, ARM64 `w1`): int/float scalars need no retain,
    /// and retaining a large 8-aligned int faults inside `fn_rc_retain`
    /// (it passes the pointer guards and reads `-16(ptr)`), while heap
    /// kinds and unknown keep the retain so aliases survive container
    /// drops. Call only when `is_tag_carrying_read(value)` holds (the tag
    /// is fresh); otherwise retain unconditionally.
    /// Implicit `: float` conversion for unknown-kind values (#83).
    /// Explicit float positions (let/assign/return/call-arg/struct-field)
    /// mark the slot Float, but an unknown-kind int value arrives
    /// unconverted and its bits are later read as a double (e.g.
    /// `4.94066e-324`). Mirrors the `float(x)` builtin exactly:
    /// statically-float and definitely-non-numeric values are untouched;
    /// tag-carrying reads (index/ternary/call) convert only when the
    /// runtime tag is not already FLOAT, everything else converts
    /// unconditionally. Value stays in rax, tag register untouched.
    pub(crate) fn emit_implicit_float_convert(&mut self, value: &crate::ast::Expr) {
        use crate::codegen::analysis::{
            is_definitely_not_numeric, is_float_expr, is_tag_carrying_read,
        };
        if is_float_expr(value, &self.ctx.variables)
            || is_definitely_not_numeric(value, &self.ctx.variables)
        {
            return;
        }
        let tag_guarded = matches!(self.arch, Architecture::X64 | Architecture::ARM64)
            && matches!(
                value,
                crate::ast::Expr::Index { .. }
                    | crate::ast::Expr::Ternary { .. }
                    | crate::ast::Expr::Call { .. }
            )
            && is_tag_carrying_read(value, &self.ctx.variables);
        if tag_guarded {
            let skip = self.ctx.next_label();
            match self.arch {
                Architecture::X64 => {
                    self.output
                        .push_str(&format!("    cmpl ${}, %edx\n", kinds::KIND_FLOAT));
                    self.output.push_str(&format!("    je {}\n", skip));
                }
                Architecture::ARM64 => {
                    self.output
                        .push_str(&format!("    cmp w1, #{}\n", kinds::KIND_FLOAT));
                    self.output.push_str(&format!("    b.eq {}\n", skip));
                }
            }
            arch::emit_int_to_float(&mut self.output, self.arch);
            self.output.push_str(&format!("{}:\n", skip));
        } else {
            arch::emit_int_to_float(&mut self.output, self.arch);
        }
    }

    /// marks the value in rax as FLOAT in the tag register. Used after an
    /// implicit conversion on tag-protocol positions (`return`), where the
    /// tag still describes the pre-conversion int.
    pub(crate) fn emit_materialize_float_tag(&mut self) {
        match self.arch {
            Architecture::X64 => {
                self.output
                    .push_str(&format!("    movl ${}, %edx\n", kinds::KIND_FLOAT));
            }
            Architecture::ARM64 => {
                self.output
                    .push_str(&format!("    mov w1, #{}\n", kinds::KIND_FLOAT));
            }
        }
    }

    /// Syncs rax into the float register (xmm0/d0). Implicit conversions
    /// leave the value in rax, but float stores (`movsd %xmm0`) and float
    /// expression users read xmm0/d0: without the sync the skip-branch
    /// (already-float value) stores stale register contents.
    pub(crate) fn emit_sync_float_reg(&mut self) {
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    movq %rax, %xmm0\n");
            }
            Architecture::ARM64 => {
                self.output.push_str("    fmov d0, x0\n");
            }
        }
    }

    pub(crate) fn emit_tag_guarded_retain(&mut self) {
        let skip = self.ctx.next_label();
        // Int/float tags need no retain; every other tag (heap, string,
        // unknown) keeps the probed call. Heap-kind tags do NOT take a
        // probe-free path: tags can go stale across overwrites that
        // don't update them, and only the probe forgives an unmapped
        // value behind a heap tag.
        match self.arch {
            Architecture::X64 => {
                self.output.push_str(&format!(
                    "    cmpl ${}, %edx\n    je {}\n    cmpl ${}, %edx\n    je {}\n",
                    kinds::KIND_INT,
                    skip,
                    kinds::KIND_FLOAT,
                    skip
                ));
            }
            Architecture::ARM64 => {
                self.output.push_str(&format!(
                    "    cmp w1, #{}\n    b.eq {}\n    cmp w1, #{}\n    b.eq {}\n",
                    kinds::KIND_INT,
                    skip,
                    kinds::KIND_FLOAT,
                    skip
                ));
            }
        }
        arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        self.output.push_str(&format!("{}:\n", skip));
    }

    /// Finds a free (non-method) function with the given bare name.
    /// Methods carry a `Type__` segment before the bare name
    /// (`Regex__is_match`, `re::Regex__is_match`); namespace-qualified
    /// globals (`re::is_match`) qualify. Prefers the current module,
    /// then the fewest qualifier segments for determinism.
    pub(crate) fn find_free_function(
        functions: &std::collections::HashSet<String>,
        bare_name: &str,
        current_fn_norm: &str,
    ) -> Option<String> {
        let cur_mod = current_fn_norm
            .split("__")
            .next()
            .unwrap_or("")
            .replace("::", "__");
        let mut best: Option<(bool, usize, String)> = None;
        for f in functions.iter() {
            let tail = f.rsplit("::").next().unwrap_or(f);
            if tail.contains("__") {
                continue;
            }
            if tail != bare_name {
                continue;
            }
            let norm = f.replace("::", "__");
            let same_mod = norm == *bare_name
                || norm.starts_with(&format!("{}__", cur_mod))
                || norm.starts_with(&format!("{}::", cur_mod));
            let segs = norm.split("__").count();
            // `same_mod` first (false sorts before true, so invert),
            // then fewest segments.
            let key = (!same_mod, segs);
            let take = match &best {
                Some((bsm, bs, _)) => key < (*bsm, *bs),
                None => true,
            };
            if take {
                best = Some((!same_mod, segs, f.clone()));
            }
        }
        best.map(|(_, _, f)| f)
    }

    pub(crate) fn emit_rodata_section(&mut self) {
        if matches!(self.os, OperatingSystem::MacOS) {
            self.output
                .push_str(".section __TEXT,__cstring,cstring_literals\n");
        } else {
            self.output.push_str(".section .rodata\n");
        }
    }

    pub(crate) fn emit_data_section(&mut self) {
        if matches!(self.os, OperatingSystem::MacOS) {
            self.output.push_str(".section __DATA,__data\n");
        } else {
            self.output.push_str(".section .data\n");
        }
    }

    pub(crate) fn emit_string_directive(&mut self, text: &str) {
        if matches!(self.os, OperatingSystem::MacOS) {
            self.output.push_str(&format!("    .asciz \"{}\"\n", text));
        } else {
            self.output.push_str(&format!("    .string \"{}\"\n", text));
        }
    }
}

pub fn generate(program: &Program, arch: Architecture, os: OperatingSystem) -> String {
    let (code, _) = generate_with_profile_ext(program, arch, os, false);
    code
}

pub fn generate_with_profile(
    program: &Program,
    arch: Architecture,
    os: OperatingSystem,
) -> (String, PipelineProfile) {
    generate_with_profile_ext(program, arch, os, false)
}

pub fn generate_with_profile_ext(
    program: &Program,
    arch: Architecture,
    os: OperatingSystem,
    no_std: bool,
) -> (String, PipelineProfile) {
    generate_full(program, arch, os, no_std, false)
}

pub fn generate_full(
    program: &Program,
    arch: Architecture,
    os: OperatingSystem,
    no_std: bool,
    mem_trace: bool,
) -> (String, PipelineProfile) {
    let mut codegen = CodeGen::new(arch, os);
    codegen.no_std = no_std;
    codegen.mem_trace = mem_trace;
    codegen.generate_program(program);
    let out = codegen.output;
    (out, codegen.profile)
}

pub fn collect_extern_libraries(program: &Program) -> Vec<String> {
    use std::collections::HashSet;
    let mut refs = HashSet::new();
    for stmt in &program.statements {
        analysis::dce::collect_references_in_stmt(stmt, &mut refs);
    }

    let mut libs = Vec::new();
    for stmt in &program.statements {
        if let Stmt::ExternBlock {
            lib: Some(ref lib),
            functions,
            ..
        } = stmt.inner_stmt()
        {
            let is_used = functions.is_empty()
                || functions.iter().any(|f| {
                    let bare = analysis::dce::bare_name(&f.name);
                    refs.contains(&f.name) || refs.contains(bare)
                });
            if is_used && !libs.contains(lib) {
                libs.push(lib.clone());
            }
        }
    }
    libs
}

fn collect_all_defers(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut defers = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Defer(inner) => {
                defers.push((**inner).clone());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                defers.extend(collect_all_defers(then_block));
                if let Some(eb) = else_block {
                    defers.extend(collect_all_defers(eb));
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body, .. }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                defers.extend(collect_all_defers(body));
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                defers.extend(collect_all_defers(try_block));
                defers.extend(collect_all_defers(catch_block));
                if let Some(fb) = finally_block {
                    defers.extend(collect_all_defers(fb));
                }
            }
            _ => {}
        }
    }
    defers
}

pub(crate) fn get_interface_flattened_methods(
    interface_name: &str,
    interfaces: &std::collections::HashMap<String, context::InterfaceDefInfo>,
) -> Vec<crate::ast::InterfaceMethod> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    fn recurse(
        name: &str,
        interfaces: &std::collections::HashMap<String, context::InterfaceDefInfo>,
        visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<crate::ast::InterfaceMethod>,
    ) {
        if !visited.insert(name.to_string()) {
            return;
        }
        if let Some(idef) = interfaces.get(name) {
            for m in &idef.methods {
                if !result.iter().any(|existing| existing.name == m.name) {
                    result.push(m.clone());
                }
            }
            for emb in &idef.embedded {
                recurse(emb, interfaces, visited, result);
            }
        }
    }
    recurse(interface_name, interfaces, &mut visited, &mut result);
    result
}
