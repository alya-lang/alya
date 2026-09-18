pub mod analysis;
pub mod arch;
pub mod context;
mod expr;
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
        }
    }

    pub fn generate_program(&mut self, program: &Program) {
        let mut resolved_prog = program.clone();
        crate::parser::enums::resolve_enums(&mut resolved_prog);
        let _ = crate::parser::constants::resolve_and_validate_constants(&mut resolved_prog);
        crate::parser::generics::resolve_generics(&mut resolved_prog);
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
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_flt:{}", f),
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
        }

        for s in &inference.known_floats {
            if s.starts_with("fn_ret_flt:")
                || s.starts_with("fn_ret_tuple_flt:")
                || s.starts_with("struct_field_flt:")
                || s.starts_with("tuple_elem_flt:")
            {
                self.ctx.variables.insert(s.clone(), VarType::Float(0));
            }
        }

        for m in &inference.known_maps {
            if m.starts_with("fn_ret_map:") {
                self.ctx.variables.insert(m.clone(), VarType::Map(0));
            }
        }

        for stmt in &program.statements {
            if let Stmt::Function {
                name,
                param_types,
                return_type,
                ..
            } = stmt.inner_stmt()
            {
                self.ctx.functions.insert(name.clone());
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
                            if bare != name {
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

                if return_type.as_deref() != Some("any") {
                    if let Some(sname) = inference.infer_function_return_struct_type(name) {
                        self.ctx.variables.insert(
                            format!("fn_ret_struct:{}", name),
                            VarType::Struct {
                                struct_name: sname.clone(),
                                offset: 0,
                            },
                        );
                        if bare != name {
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
                    self.ctx.functions.contains(&c1) || self.ctx.functions.contains(&c2)
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
                            if bare != f.name {
                                self.ctx.variables.insert(
                                    format!("fn_ret_str:{}", bare),
                                    VarType::StringOffset(0),
                                );
                            }
                        } else if ret_type == "float" || ret_type == "f64" || ret_type == "f32" {
                            self.ctx
                                .variables
                                .insert(format!("fn_ret_flt:{}", f.name), VarType::Float(0));
                            if bare != f.name {
                                self.ctx
                                    .variables
                                    .insert(format!("fn_ret_flt:{}", bare), VarType::Float(0));
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

        if !self.ctx.globals.is_empty() {
            self.emit_data_section();
            let word_dir = if matches!(self.arch, Architecture::X86) {
                ".long"
            } else {
                ".quad"
            };
            for (symbol, _) in self.ctx.globals.values() {
                self.output.push_str(&format!(
                    ".global {}\n{}:\n    {} 0\n",
                    symbol, symbol, word_dir
                ));
            }
            self.output.push_str(".text\n");
        }

        self.emit_rodata_section();
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

        for stmt in top_level {
            self.generate_statement(stmt);
        }

        self.emit_cleanup_scope(None);

        arch::emit_footer(&mut self.output, self.arch);

        for func in functions {
            if let Stmt::Function {
                name,
                params,
                param_types,
                body,
                ..
            } = func
            {
                self.generate_function(name, params, param_types, body, program, &inference);
            }
        }

        if !self.no_std {
            runtime::emit_runtime(
                &mut self.output,
                self.arch,
                self.os,
                &self.ctx.structs,
                &self.ctx.interfaces,
                &self.ctx.vtables,
            );
        }
        let d_codegen = t_emit.elapsed();

        self.profile = PipelineProfile {
            d_call_index,
            d_dce,
            d_inference,
            d_codegen,
            original_stmts,
            pruned_stmts,
        };
    }

    fn generate_function(
        &mut self,
        name: &str,
        params: &[String],
        param_types: &[Option<String>],
        body: &[Stmt],
        program: &Program,
        inference: &ProgramInference,
    ) {
        let saved = self.ctx.enter_function();
        self.ctx.current_fn_name = name.to_string();

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

        arch::emit_function_prologue(&mut self.output, self.arch, name);

        let mut heap_param_offsets = Vec::new();
        for (i, param) in params.iter().enumerate() {
            arch::emit_function_param_push(
                &mut self.output,
                self.arch,
                i,
                &mut self.ctx.stack_offset,
                self.os,
            );

            let is_str = inference.infer_param_is_string(name, i, program);
            let is_flt = inference.infer_param_is_float(name, i, program);
            let is_arr = inference.infer_param_is_array(name, i, program)
                || param_types
                    .get(i)
                    .and_then(|t| t.as_deref())
                    .map_or(false, |t| t == "..." || t.starts_with("...") || t.ends_with("[]"));
            let is_str_arr = inference.infer_param_is_string_array(name, i, program);
            let is_flt_arr = inference.infer_param_is_float_array(name, i, program);
            let is_map = inference.infer_param_is_map(name, i, program);
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
            } else if is_flt {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Float(self.ctx.stack_offset));
            } else if is_arr || is_str_arr || is_flt_arr {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Array(self.ctx.stack_offset));
                if is_str_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_str:{}", param), VarType::Number(0));
                }
                if is_flt_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_flt:{}", param), VarType::Number(0));
                }
            } else if is_map {
                self.ctx
                    .variables
                    .insert(param.clone(), VarType::Map(self.ctx.stack_offset));
            } else {
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
                heap_param_offsets.push(self.ctx.stack_offset);
            }
        }

        for offset in heap_param_offsets {
            arch::emit_load_var(&mut self.output, self.arch, offset, self.ctx.stack_offset);
            arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
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

        arch::emit_function_epilogue(&mut self.output, self.arch);

        self.ctx.exit_function(saved);
    }

    pub(crate) fn get_scope_heap_offsets(&self, skip_offset: Option<i32>) -> Vec<i32> {
        let mut offsets: Vec<i32> = self
            .ctx
            .variables
            .iter()
            .filter(|(name, _)| !name.contains(':') && !name.contains('.'))
            .filter_map(|(_, vtype)| match vtype {
                VarType::Array(off) | VarType::Map(off) | VarType::Struct { offset: off, .. } => {
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

    pub(crate) fn emit_cleanup_scope(&mut self, skip_offset: Option<i32>) {
        let offsets = self.get_scope_heap_offsets(skip_offset);
        for offset in offsets {
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                offset,
                self.ctx.stack_offset,
                self.os,
            );
        }
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
                    return Some(bare.to_string());
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
            Expr::Index { array, .. } => {
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
            _ => false,
        }
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
    let mut codegen = CodeGen::new(arch, os);
    codegen.no_std = no_std;
    codegen.generate_program(program);
    (codegen.output, codegen.profile)
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
