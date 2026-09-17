use super::CodeGen;
use crate::ast::Expr;
use crate::codegen::analysis::{
    escape_string, is_array_expr, is_float_array, is_float_expr, is_map_expr, is_null_expr,
    is_string_array, is_string_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

impl CodeGen {
    pub(super) fn generate_let(&mut self, name: &str, type_ann: Option<&str>, value: &Expr) {
        let name = name.to_string();
        match value {
            Expr::Null => {
                self.generate_expression(value);
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::Null(self.ctx.stack_offset));
            }
            Expr::String(s) => {
                let label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", label));
                self.emit_string_directive(&escape_string(s));
                self.output.push_str(".text\n");
                arch::emit_load_str_label(&mut self.output, self.arch, &label, self.os);
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
            }
            Expr::Array(elements) => {
                let is_str_arr = (!elements.is_empty()
                    && elements
                        .iter()
                        .all(|e| is_string_expr(e, &self.ctx.variables)))
                    || matches!(type_ann, Some("string[]") | Some("str[]"));
                let is_flt_arr = elements
                    .first()
                    .is_some_and(|e| is_float_expr(e, &self.ctx.variables))
                    || matches!(type_ann, Some("float[]") | Some("f64[]"));
                let struct_elem_type = if let Some(t) = type_ann.and_then(|t| t.strip_suffix("[]"))
                {
                    if self.ctx.structs.contains_key(t) {
                        Some(t.to_string())
                    } else {
                        None
                    }
                } else {
                    elements.first().and_then(|e| match e {
                        Expr::StructInit { name, .. } => Some(name.clone()),
                        Expr::Call { name, .. } if self.ctx.structs.contains_key(name) => {
                            Some(name.clone())
                        }
                        Expr::Identifier(id) => match self.ctx.variables.get(id) {
                            Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                            _ => None,
                        },
                        _ => None,
                    })
                };
                self.generate_expression(value);

                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);

                self.ctx
                    .variables
                    .insert(name.clone(), VarType::Array(self.ctx.stack_offset));
                if let Some(sname) = struct_elem_type {
                    self.ctx.variables.insert(
                        format!("arr_struct_type:{}", name),
                        VarType::Struct {
                            struct_name: sname,
                            offset: 0,
                        },
                    );
                }
                if is_str_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_str:{}", name), VarType::Number(0));
                }
                if is_flt_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_flt:{}", name), VarType::Number(0));
                }
            }
            Expr::StructInit {
                name: sname,
                fields: init_fields,
            } => {
                self.generate_expression(value);

                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);

                self.ctx.variables.insert(
                    name.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: self.ctx.stack_offset,
                    },
                );

                for (fname, fval) in init_fields {
                    let is_flt = is_float_expr(fval, &self.ctx.variables);
                    let is_str = is_string_expr(fval, &self.ctx.variables);
                    let is_arr = is_array_expr(fval, &self.ctx.variables);
                    let is_map = is_map_expr(fval, &self.ctx.variables);
                    let field_key = format!("{}.{}", name, fname);
                    if is_str {
                        self.ctx
                            .variables
                            .insert(field_key, VarType::StringOffset(0));
                        self.ctx.variables.insert(
                            format!("struct_field_str:{}.{}", sname, fname),
                            VarType::StringOffset(0),
                        );
                        self.ctx.variables.insert(
                            format!("struct_field_str:{}", fname),
                            VarType::StringOffset(0),
                        );
                    } else if is_flt {
                        self.ctx.variables.insert(field_key, VarType::Float(0));
                        self.ctx.variables.insert(
                            format!("struct_field_flt:{}.{}", sname, fname),
                            VarType::Float(0),
                        );
                        self.ctx
                            .variables
                            .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                    } else if is_arr {
                        self.ctx.variables.insert(field_key, VarType::Array(0));
                        self.ctx.variables.insert(
                            format!("struct_field_arr:{}.{}", sname, fname),
                            VarType::Array(0),
                        );
                        self.ctx
                            .variables
                            .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                        if is_string_array(fval, &self.ctx.variables) {
                            self.ctx.variables.insert(
                                format!("struct_field_arr_str:{}.{}", sname, fname),
                                VarType::Number(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_arr_str:{}", fname),
                                VarType::Number(0),
                            );
                        }
                    } else if is_map {
                        self.ctx.variables.insert(field_key, VarType::Map(0));
                        self.ctx.variables.insert(
                            format!("struct_field_map:{}.{}", sname, fname),
                            VarType::Map(0),
                        );
                        self.ctx
                            .variables
                            .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                    } else {
                        self.ctx.variables.insert(field_key, VarType::Number(0));
                    }
                }
            }
            Expr::Call {
                name: cname,
                args: cargs,
            } if self.ctx.structs.contains_key(cname) => {
                let sname = cname.clone();
                let sfields = self
                    .ctx
                    .structs
                    .get(&sname)
                    .map(|s| s.fields.clone())
                    .unwrap_or_default();

                self.generate_expression(value);

                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);

                self.ctx.variables.insert(
                    name.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: self.ctx.stack_offset,
                    },
                );

                for (i, arg) in cargs.iter().enumerate() {
                    if let Some(fname) = sfields.get(i) {
                        let is_flt = is_float_expr(arg, &self.ctx.variables);
                        let is_str = is_string_expr(arg, &self.ctx.variables);
                        let is_arr = is_array_expr(arg, &self.ctx.variables);
                        let is_map = is_map_expr(arg, &self.ctx.variables);
                        let field_key = format!("{}.{}", name, fname);
                        if is_str {
                            self.ctx
                                .variables
                                .insert(field_key, VarType::StringOffset(0));
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", sname, fname),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", fname),
                                VarType::StringOffset(0),
                            );
                        } else if is_flt {
                            self.ctx.variables.insert(field_key, VarType::Float(0));
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", sname, fname),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                        } else if is_arr {
                            self.ctx.variables.insert(field_key, VarType::Array(0));
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", sname, fname),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                            if is_string_array(arg, &self.ctx.variables) {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_str:{}.{}", sname, fname),
                                    VarType::Number(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr_str:{}", fname),
                                    VarType::Number(0),
                                );
                            }
                        } else if is_map {
                            self.ctx.variables.insert(field_key, VarType::Map(0));
                            self.ctx.variables.insert(
                                format!("struct_field_map:{}.{}", sname, fname),
                                VarType::Map(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                        } else {
                            self.ctx.variables.insert(field_key, VarType::Number(0));
                        }
                    }
                }
            }
            _ => {
                let is_explicit_str = matches!(type_ann, Some("string") | Some("str"));
                let is_explicit_flt = matches!(type_ann, Some("float") | Some("f64"));
                let is_explicit_arr = matches!(type_ann, Some("array"))
                    || type_ann.is_some_and(|t| t.ends_with("[]"));
                let is_explicit_map = matches!(type_ann, Some("map"));

                let is_struct_from_ann = if let Some(t) = type_ann {
                    let bare = t.rsplit("::").next().unwrap_or(t);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if self.ctx.structs.contains_key(t) {
                        Some(t.to_string())
                    } else if self.ctx.structs.contains_key(bare) {
                        Some(bare.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let is_str = is_explicit_str || is_string_expr(value, &self.ctx.variables);
                let is_arr = is_explicit_arr || is_array_expr(value, &self.ctx.variables);
                let is_struct = is_struct_from_ann.or_else(|| self.get_expr_struct_name(value));
                let is_flt = is_explicit_flt || is_float_expr(value, &self.ctx.variables);
                let is_map = is_explicit_map || is_map_expr(value, &self.ctx.variables);
                let is_null = is_null_expr(value, &self.ctx.variables);
                let is_alias_heap = match value {
                    Expr::Identifier(ident) => matches!(
                        self.ctx.variables.get(ident),
                        Some(VarType::Array(_))
                            | Some(VarType::Map(_))
                            | Some(VarType::Struct { .. })
                    ),
                    Expr::Index { .. } | Expr::FieldAccess { .. } => {
                        is_arr || is_map || is_struct.is_some()
                    }
                    _ => false,
                };
                self.generate_expression(value);

                if is_alias_heap {
                    arch::emit_rc_retain(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }

                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);

                if let Some(sname) = is_struct {
                    self.ctx.variables.insert(
                        name.clone(),
                        VarType::Struct {
                            struct_name: sname,
                            offset: self.ctx.stack_offset,
                        },
                    );
                } else if is_map {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Map(self.ctx.stack_offset));
                    if let Expr::Map(entries) = value {
                        for (k, v) in entries {
                            if is_string_expr(v, &self.ctx.variables) {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_field_str:{}", field),
                                        VarType::StringOffset(0),
                                    );
                                    self.ctx.variables.insert(
                                        format!("map_str:{}.{}", name, field),
                                        VarType::StringOffset(0),
                                    );
                                }
                            }
                            if is_map_expr(v, &self.ctx.variables) {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_field_map:{}", field),
                                        VarType::Map(0),
                                    );
                                    self.ctx.variables.insert(
                                        format!("map_map:{}.{}", name, field),
                                        VarType::Map(0),
                                    );
                                }
                            }
                        }
                    }
                } else if is_arr {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Array(self.ctx.stack_offset));
                    if matches!(type_ann, Some("string[]") | Some("str[]"))
                        || is_string_array(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("arr_is_str:{}", name), VarType::Number(0));
                    }
                    if matches!(type_ann, Some("float[]") | Some("f64[]"))
                        || is_float_array(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("arr_is_flt:{}", name), VarType::Number(0));
                    }
                } else if is_str {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
                } else if is_flt {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Float(self.ctx.stack_offset));
                } else if is_null {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Null(self.ctx.stack_offset));
                } else {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Number(self.ctx.stack_offset));
                }

                if let Expr::Identifier(target_fn) = value {
                    let bare_tgt = target_fn.rsplit("::").next().unwrap_or(target_fn);
                    let bare_tgt = bare_tgt.rsplit("__").next().unwrap_or(bare_tgt);
                    for ret_prefix in &[
                        "fn_ret_str:",
                        "fn_ret_flt:",
                        "fn_ret_map:",
                        "fn_ret_arr:",
                        "fn_ret_struct:",
                    ] {
                        let k1 = format!("{}{}", ret_prefix, target_fn);
                        let k2 = format!("{}{}", ret_prefix, bare_tgt);
                        if let Some(vt) = self
                            .ctx
                            .variables
                            .get(&k1)
                            .or_else(|| self.ctx.variables.get(&k2))
                            .cloned()
                        {
                            self.ctx
                                .variables
                                .insert(format!("{}{}", ret_prefix, name), vt);
                        }
                    }
                }

                if let Expr::Call { name: cname, .. } = value {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let prefix1 = format!("fn_ret_tuple_str:{}:", cname);
                    let prefix2 = format!("fn_ret_tuple_str:{}:", bare);
                    let matching: Vec<(String, String)> = self
                        .ctx
                        .variables
                        .keys()
                        .filter(|k| k.starts_with(&prefix1) || k.starts_with(&prefix2))
                        .filter_map(|k| {
                            k.strip_prefix(&prefix1)
                                .or_else(|| k.strip_prefix(&prefix2))
                                .map(|idx| (name.clone(), idx.to_string()))
                        })
                        .collect();
                    for (arr_name, idx_str) in matching {
                        self.ctx.variables.insert(
                            format!("tuple_elem_str:{}:{}", arr_name, idx_str),
                            VarType::StringOffset(0),
                        );
                    }
                } else if let Expr::Array(elems) = value {
                    for (i, elem) in elems.iter().enumerate() {
                        if is_string_expr(elem, &self.ctx.variables) {
                            self.ctx.variables.insert(
                                format!("tuple_elem_str:{}:{}", name, i),
                                VarType::StringOffset(0),
                            );
                        }
                    }
                }
            }
        }
    }

    pub(super) fn generate_assign(&mut self, name: &str, value: &Expr) {
        let name = name.to_string();
        let is_flt = is_float_expr(value, &self.ctx.variables);
        let is_str = is_string_expr(value, &self.ctx.variables);
        let is_arr = is_array_expr(value, &self.ctx.variables);
        let is_map = is_map_expr(value, &self.ctx.variables);
        let is_null = is_null_expr(value, &self.ctx.variables);
        let old_heap_offset = match self.ctx.variables.get(&name) {
            Some(VarType::Array(off))
            | Some(VarType::Map(off))
            | Some(VarType::Struct { offset: off, .. }) => {
                if *off > 0 {
                    Some(*off)
                } else {
                    None
                }
            }
            _ => None,
        };
        let is_alias_heap = match value {
            Expr::Identifier(ident) => matches!(
                self.ctx.variables.get(ident),
                Some(VarType::Array(_)) | Some(VarType::Map(_)) | Some(VarType::Struct { .. })
            ),
            Expr::Index { .. } | Expr::FieldAccess { .. } => {
                old_heap_offset.is_some() || is_arr || is_map
            }
            _ => false,
        };

        self.generate_expression(value);

        if is_alias_heap {
            arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }

        if let Some(old_offset) = old_heap_offset {
            arch::emit_push_temp(&mut self.output, self.arch);
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                old_offset,
                self.ctx.stack_offset + 8,
                self.os,
            );
            arch::emit_pop_temp(&mut self.output, self.arch);
            if is_flt {
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq %rax, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    fmov d0, x0\n");
                    }
                    Architecture::X86 => {}
                }
            }
        }

        if let Some(var_type) = self.ctx.variables.get(&name).cloned() {
            match var_type {
                VarType::Number(offset)
                | VarType::Float(offset)
                | VarType::StringOffset(offset)
                | VarType::Array(offset)
                | VarType::Map(offset)
                | VarType::Null(offset)
                | VarType::Struct { offset, .. } => {
                    if is_flt {
                        arch::emit_store_var_float(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                        );
                    } else {
                        arch::emit_store_var(
                            &mut self.output,
                            self.arch,
                            offset,
                            self.ctx.stack_offset,
                        );
                    }
                    if is_null {
                        self.ctx
                            .variables
                            .insert(name.clone(), VarType::Null(offset));
                    } else if is_map {
                        self.ctx
                            .variables
                            .insert(name.clone(), VarType::Map(offset));
                        if let Expr::Map(entries) = value {
                            for (k, v) in entries {
                                if is_string_expr(v, &self.ctx.variables) {
                                    if let Expr::String(field) = k {
                                        self.ctx.variables.insert(
                                            format!("map_field_str:{}", field),
                                            VarType::StringOffset(0),
                                        );
                                        self.ctx.variables.insert(
                                            format!("map_str:{}.{}", name, field),
                                            VarType::StringOffset(0),
                                        );
                                    }
                                }
                                if is_map_expr(v, &self.ctx.variables) {
                                    if let Expr::String(field) = k {
                                        self.ctx.variables.insert(
                                            format!("map_field_map:{}", field),
                                            VarType::Map(0),
                                        );
                                        self.ctx.variables.insert(
                                            format!("map_map:{}.{}", name, field),
                                            VarType::Map(0),
                                        );
                                    }
                                }
                            }
                        }
                    } else if is_arr {
                        self.ctx
                            .variables
                            .insert(name.clone(), VarType::Array(offset));
                        if is_string_array(value, &self.ctx.variables) {
                            self.ctx
                                .variables
                                .insert(format!("arr_is_str:{}", name), VarType::Number(0));
                        }
                        if is_float_array(value, &self.ctx.variables) {
                            self.ctx
                                .variables
                                .insert(format!("arr_is_flt:{}", name), VarType::Number(0));
                        }
                    } else if is_str {
                        self.ctx
                            .variables
                            .insert(name.clone(), VarType::StringOffset(offset));
                    } else if is_flt {
                        self.ctx
                            .variables
                            .insert(name.clone(), VarType::Float(offset));
                    } else {
                        let is_struct = self.get_expr_struct_name(value);
                        if let Some(sname) = is_struct {
                            self.ctx.variables.insert(
                                name.clone(),
                                VarType::Struct {
                                    struct_name: sname,
                                    offset,
                                },
                            );
                        } else {
                            self.ctx
                                .variables
                                .insert(name.clone(), VarType::Number(offset));
                        }
                    }
                }
                VarType::StringLabel(_) => {}
            }
        }

        if let Expr::Call { name: cname, .. } = value {
            let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            let prefix1 = format!("fn_ret_tuple_str:{}:", cname);
            let prefix2 = format!("fn_ret_tuple_str:{}:", bare);
            let matching: Vec<(String, String)> = self
                .ctx
                .variables
                .keys()
                .filter(|k| k.starts_with(&prefix1) || k.starts_with(&prefix2))
                .filter_map(|k| {
                    k.strip_prefix(&prefix1)
                        .or_else(|| k.strip_prefix(&prefix2))
                        .map(|idx| (name.clone(), idx.to_string()))
                })
                .collect();
            for (arr_name, idx_str) in matching {
                self.ctx.variables.insert(
                    format!("tuple_elem_str:{}:{}", arr_name, idx_str),
                    VarType::StringOffset(0),
                );
            }
        } else if let Expr::Array(elems) = value {
            for (i, elem) in elems.iter().enumerate() {
                if is_string_expr(elem, &self.ctx.variables) {
                    self.ctx.variables.insert(
                        format!("tuple_elem_str:{}:{}", name, i),
                        VarType::StringOffset(0),
                    );
                }
            }
        }
    }

    pub(super) fn generate_field_assign(&mut self, object: &Expr, field: &str, value: &Expr) {
        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            _ => object,
        };

        if let Expr::Identifier(obj_name) = base_obj {
            let field_key = format!("{}.{}", obj_name, field);
            let is_flt = is_float_expr(value, &self.ctx.variables);
            let is_str = is_string_expr(value, &self.ctx.variables);
            let is_null = is_null_expr(value, &self.ctx.variables);
            if is_str {
                self.ctx
                    .variables
                    .insert(field_key, VarType::StringOffset(0));
            } else if is_flt {
                self.ctx.variables.insert(field_key, VarType::Float(0));
            } else if is_null {
                self.ctx.variables.insert(field_key, VarType::Null(0));
            } else {
                self.ctx.variables.insert(field_key, VarType::Number(0));
            }
        }

        let field_idx = self.resolve_struct_field_index(object, field);

        self.generate_expression(object);
        arch::emit_push_temp(&mut self.output, self.arch);

        self.generate_expression(value);
        if self.is_heap_expression(value) {
            arch::emit_rc_retain(
                &mut self.output,
                self.arch,
                self.ctx.stack_offset + 8,
                self.os,
            );
        }
        arch::emit_struct_field_set(&mut self.output, self.arch, field_idx);
    }

    pub(super) fn generate_index_assign(&mut self, array: &Expr, index: &Expr, value: &Expr) {
        if is_map_expr(array, &self.ctx.variables)
            || is_string_expr(index, &self.ctx.variables)
            || matches!(index, Expr::String(_))
        {
            let actual_args = [array, index, value];
            match self.arch {
                Architecture::X86 => {
                    for arg in actual_args.iter().rev() {
                        self.generate_expression(arg);
                        if std::ptr::eq(*arg, value) && self.is_heap_expression(value) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }
                }
                _ => {
                    for arg in actual_args.iter() {
                        self.generate_expression(arg);
                        if std::ptr::eq(*arg, value) && self.is_heap_expression(value) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }
                }
            }
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "set",
                3,
                self.ctx.stack_offset,
                self.os,
            );
            if is_string_expr(value, &self.ctx.variables) {
                if let Expr::String(field) = index {
                    self.ctx
                        .variables
                        .insert(format!("map_field_str:{}", field), VarType::StringOffset(0));
                }
                if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                    self.ctx.variables.insert(
                        format!("map_str:{}.{}", map_name, field),
                        VarType::StringOffset(0),
                    );
                }
            }
            if is_map_expr(value, &self.ctx.variables) {
                if let Expr::String(field) = index {
                    self.ctx
                        .variables
                        .insert(format!("map_field_map:{}", field), VarType::Map(0));
                }
                if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                    self.ctx
                        .variables
                        .insert(format!("map_map:{}.{}", map_name, field), VarType::Map(0));
                }
            }
        } else {
            self.generate_expression(array);
            arch::emit_push_temp(&mut self.output, self.arch);

            self.generate_expression(index);
            arch::emit_push_temp(&mut self.output, self.arch);

            self.generate_expression(value);
            if self.is_heap_expression(value) {
                arch::emit_rc_retain(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset + 16,
                    self.os,
                );
            }
            arch::emit_array_set(&mut self.output, self.arch);
        }
    }
}
