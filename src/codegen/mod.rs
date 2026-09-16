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

pub struct CodeGen {
    pub(crate) arch: Architecture,
    pub(crate) os: OperatingSystem,
    pub(crate) output: String,
    pub(crate) ctx: CodeGenContext,
}

impl CodeGen {
    pub fn new(arch: Architecture, os: OperatingSystem) -> Self {
        Self {
            arch,
            os,
            output: String::new(),
            ctx: CodeGenContext::new(),
        }
    }

    pub fn generate_program(&mut self, program: &Program) {
        let mut resolved_prog;
        let program = if program
            .statements
            .iter()
            .any(|s| matches!(s, Stmt::EnumDef { .. } | Stmt::Const { .. }))
        {
            resolved_prog = program.clone();
            crate::parser::enums::resolve_enums(&mut resolved_prog);
            let _ = crate::parser::constants::resolve_and_validate_constants(&mut resolved_prog);
            &resolved_prog
        } else {
            program
        };

        // Collect all struct definitions first
        for stmt in &program.statements {
            if let Stmt::StructDef {
                name,
                fields,
                defaults,
            } = stmt
            {
                self.ctx.structs.insert(
                    name.clone(),
                    context::StructDefInfo {
                        name: name.clone(),
                        fields: fields.clone(),
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
                            defaults: defaults.clone(),
                        },
                    );
                }
            }
        }

        let inference = ProgramInference::analyze(program);
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
            if s.starts_with("fn_ret_flt:") || s.starts_with("struct_field_flt:") {
                self.ctx.variables.insert(s.clone(), VarType::Float(0));
            }
        }

        for m in &inference.known_maps {
            if m.starts_with("fn_ret_map:") {
                self.ctx.variables.insert(m.clone(), VarType::Map(0));
            }
        }

        for stmt in &program.statements {
            if let Stmt::Function { name, .. } = stmt {
                self.ctx.functions.insert(name.clone());
                if let Some(sname) = inference.infer_function_return_struct_type(name) {
                    self.ctx.variables.insert(
                        format!("fn_ret_struct:{}", name),
                        VarType::Struct {
                            struct_name: sname.clone(),
                            offset: 0,
                        },
                    );
                    let bare = name.rsplit("::").next().unwrap_or(name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
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

        let struct_inf = &inference.struct_inf;
        for ((sname, fname), inner_st) in &struct_inf.field_types {
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

        // Collect all extern declarations
        for stmt in &program.statements {
            if let Stmt::ExternBlock {
                abi,
                lib,
                functions,
            } = stmt
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
            match stmt {
                Stmt::Function { .. } => functions.push(stmt),
                _ => top_level.push(stmt),
            }
        }

        arch::emit_header(&mut self.output, self.arch, self.os);

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

        runtime::emit_runtime(&mut self.output, self.arch, self.os, &self.ctx.structs);
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
                || param_types.get(i).and_then(|t| t.as_deref()) == Some("...");
            let is_str_arr = inference.infer_param_is_string_array(name, i, program);
            let is_flt_arr = inference.infer_param_is_float_array(name, i, program);
            let is_map = inference.infer_param_is_map(name, i, program);
            let struct_type = inference.infer_param_struct_type(name, i);
            if let Some(ref sname) = struct_type {
                self.ctx.variables.insert(
                    param.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
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

            let is_heap_param =
                struct_type.is_some() || is_arr || is_str_arr || is_flt_arr || is_map;
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

    pub(crate) fn is_heap_expression(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr;
        match expr {
            Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } => true,
            Expr::Identifier(name) => matches!(
                self.ctx.variables.get(name),
                Some(VarType::Array(_)) | Some(VarType::Map(_)) | Some(VarType::Struct { .. })
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
                crate::codegen::analysis::is_array_expr(expr, &self.ctx.variables)
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

    pub(crate) fn emit_string_directive(&mut self, text: &str) {
        if matches!(self.os, OperatingSystem::MacOS) {
            self.output.push_str(&format!("    .asciz \"{}\"\n", text));
        } else {
            self.output.push_str(&format!("    .string \"{}\"\n", text));
        }
    }
}

pub fn generate(program: &Program, arch: Architecture, os: OperatingSystem) -> String {
    let mut codegen = CodeGen::new(arch, os);
    codegen.generate_program(program);
    codegen.output
}

pub fn collect_extern_libraries(program: &Program) -> Vec<String> {
    let mut libs = Vec::new();
    for stmt in &program.statements {
        if let Stmt::ExternBlock {
            lib: Some(ref lib), ..
        } = stmt
        {
            if !libs.contains(lib) {
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
