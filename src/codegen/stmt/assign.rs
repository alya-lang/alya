use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    escape_string, is_array_expr, is_float_array, is_float_expr, is_map_expr, is_null_expr,
    is_number_expr, is_string_array, is_string_expr, is_tag_carrying_read, is_unsigned_expr,
    string_store_needs_dup, struct_field_markers_mixed_vars, value_kind_tag,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

impl CodeGen {
    pub(super) fn generate_let(&mut self, name: &str, type_ann: Option<&str>, value: &Expr) {
        let name = name.to_string();
        if self.ctx.current_fn_name.is_empty() {
            if let Some((symbol, sname)) = self.ctx.globals.get(&name).cloned() {
                self.generate_expression(value);
                // B1: named stores outlive the wrapping ring buffer.
                if string_store_needs_dup(value, &self.ctx.variables) {
                    arch::emit_str_store(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
                if self.is_heap_expression(value) {
                    arch::emit_rc_retain(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
                arch::emit_store_global(&mut self.output, self.arch, &symbol, self.os);
                if let Some(sn) = sname {
                    self.ctx.variables.insert(
                        name.clone(),
                        VarType::Struct {
                            struct_name: sn,
                            offset: 0,
                        },
                    );
                } else if is_string_expr(value, &self.ctx.variables) {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::StringOffset(0));
                } else if is_float_expr(value, &self.ctx.variables) {
                    self.ctx.variables.insert(name.clone(), VarType::Float(0));
                } else {
                    self.ctx.variables.insert(name.clone(), VarType::Number(0));
                    if matches!(
                        type_ann,
                        Some("int")
                            | Some("i64")
                            | Some("i32")
                            | Some("u64")
                            | Some("u32")
                            | Some("uint")
                            | Some("byte")
                            | Some("char")
                            | Some("bool")
                    ) || matches!(value, Expr::Number(_))
                        || is_number_expr(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", name), VarType::Number(0));
                    }
                    // Unsigned marker for u64-family annotations (B4):
                    // drives unsigned div/mod/cmp/display. Kept alongside
                    // var_is_int (which means "integer-like" for existing
                    // optimizations); unsigned wins where they differ.
                    // Propagates through unsigned values (let y = x) so
                    // annotation need not repeat on every derived local.
                    if matches!(
                        type_ann,
                        Some("u64") | Some("u32") | Some("uint") | Some("usize")
                    ) || is_unsigned_expr(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_uint:{}", name), VarType::Number(0));
                    }
                }
                if let Some(t) = type_ann {
                    if t.starts_with("Channel[string]") || t.starts_with("Channel[str]") {
                        self.ctx
                            .variables
                            .insert(format!("channel_elem_str:{}", name), VarType::Number(0));
                    }
                }
                if let Expr::Call {
                    name: cname,
                    args: cargs,
                } = value
                {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if bare == "new" {
                        if let Some(Expr::Index { array, index }) = cargs.first() {
                            if let (Expr::Identifier(arr_id), Expr::Identifier(type_id)) =
                                (&**array, &**index)
                            {
                                if arr_id == "Channel" && (type_id == "string" || type_id == "str")
                                {
                                    self.ctx.variables.insert(
                                        format!("channel_elem_str:{}", name),
                                        VarType::Number(0),
                                    );
                                }
                            }
                        }
                    }
                }
                if let Expr::Array(elems) = value {
                    for (i, elem) in elems.iter().enumerate() {
                        if is_string_expr(elem, &self.ctx.variables) {
                            self.ctx.variables.insert(
                                format!("tuple_elem_str:{}:{}", name, i),
                                VarType::StringOffset(0),
                            );
                        }
                        if is_float_expr(elem, &self.ctx.variables) {
                            self.ctx.variables.insert(
                                format!("tuple_elem_flt:{}:{}", name, i),
                                VarType::Float(0),
                            );
                        }
                    }
                }
                if let Some(ann) = type_ann {
                    let ann = ann.trim();
                    if ann.starts_with('(') && ann.ends_with(')') {
                        for (i, ty) in ann[1..ann.len() - 1].split(',').enumerate() {
                            let ty = ty.trim();
                            if ty == "string" || ty == "str" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_str:{}:{}", name, i),
                                    VarType::StringOffset(0),
                                );
                            } else if ty == "float" || ty == "f64" || ty == "f32" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_flt:{}:{}", name, i),
                                    VarType::Float(0),
                                );
                            }
                        }
                    }
                }
                return;
            }
        }
        // Rebinding an existing heap variable (e.g. `let` redeclaration in
        // a loop body) must drop the previous value first; otherwise every
        // iteration leaks it (alya-lang/alya#79). Mirrors generate_assign.
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
        match value {
            Expr::Null => {
                self.generate_expression(value);
                self.emit_let_rebind_release(old_heap_offset, false);
                // Rebinding null needs no store: the rebind release
                // already nulled the old slot.
                let slot = match old_heap_offset {
                    Some(off) if off > 0 => off,
                    _ => {
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        self.ctx.stack_offset
                    }
                };
                self.ctx.variables.insert(name.clone(), VarType::Null(slot));
            }
            Expr::String(s) => {
                let label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", label));
                self.emit_string_directive(&escape_string(s));
                self.output.push_str(".text\n");
                arch::emit_load_str_label(&mut self.output, self.arch, &label, self.os);
                self.emit_let_rebind_release(old_heap_offset, false);
                let slot = self.let_home_slot(old_heap_offset, false);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::StringOffset(slot));
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
                // Explicit integer-array annotations are enforced by the
                // type checker, so they are exact element-kind facts.
                // Literal-based inference would go stale on later pushes
                // of other kinds, so only the annotation qualifies.
                let is_int_arr =
                    type_ann.is_some_and(crate::codegen::analysis::is_int_array_annotation);
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

                self.emit_let_rebind_release(old_heap_offset, false);
                let slot = self.let_home_slot(old_heap_offset, false);

                self.ctx
                    .variables
                    .insert(name.clone(), VarType::Array(slot));
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
                if is_int_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_is_int:{}", name), VarType::Number(0));
                }
                for (i, elem) in elements.iter().enumerate() {
                    if is_string_expr(elem, &self.ctx.variables) {
                        self.ctx.variables.insert(
                            format!("tuple_elem_str:{}:{}", name, i),
                            VarType::StringOffset(0),
                        );
                    }
                    if is_float_expr(elem, &self.ctx.variables) {
                        self.ctx
                            .variables
                            .insert(format!("tuple_elem_flt:{}:{}", name, i), VarType::Float(0));
                    }
                }
                if let Some(ann) = type_ann {
                    let ann = ann.trim();
                    if ann.starts_with('(') && ann.ends_with(')') {
                        for (i, ty) in ann[1..ann.len() - 1].split(',').enumerate() {
                            let ty = ty.trim();
                            if ty == "string" || ty == "str" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_str:{}:{}", name, i),
                                    VarType::StringOffset(0),
                                );
                            } else if ty == "float" || ty == "f64" || ty == "f32" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_flt:{}:{}", name, i),
                                    VarType::Float(0),
                                );
                            }
                        }
                    }
                }
            }
            Expr::StructInit {
                name: sname,
                fields: init_fields,
            } => {
                self.generate_expression(value);

                self.emit_let_rebind_release(old_heap_offset, false);
                let slot = self.let_home_slot(old_heap_offset, false);

                self.ctx.variables.insert(
                    name.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: slot,
                    },
                );

                for (fname, fval) in init_fields {
                    let is_flt = is_float_expr(fval, &self.ctx.variables);
                    let is_str = is_string_expr(fval, &self.ctx.variables);
                    let is_arr = is_array_expr(fval, &self.ctx.variables);
                    let is_map = is_map_expr(fval, &self.ctx.variables);
                    let field_key = format!("{}.{}", name, fname);
                    // Mixed literal kinds for this field: skip global markers;
                    // per-variable field_key stays precise.
                    let sf_mixed =
                        struct_field_markers_mixed_vars(&self.ctx.variables, sname, fname);
                    if is_str {
                        self.ctx
                            .variables
                            .insert(field_key, VarType::StringOffset(0));
                        if !sf_mixed {
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", sname, fname),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", fname),
                                VarType::StringOffset(0),
                            );
                        }
                    } else if is_flt {
                        self.ctx.variables.insert(field_key, VarType::Float(0));
                        if !sf_mixed {
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", sname, fname),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                        }
                    } else if is_arr {
                        self.ctx.variables.insert(field_key, VarType::Array(0));
                        if !sf_mixed {
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", sname, fname),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                        }
                        if is_string_array(fval, &self.ctx.variables) && !sf_mixed {
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
                        if !sf_mixed {
                            self.ctx.variables.insert(
                                format!("struct_field_map:{}.{}", sname, fname),
                                VarType::Map(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                        }
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

                self.emit_let_rebind_release(old_heap_offset, false);
                let slot = self.let_home_slot(old_heap_offset, false);

                self.ctx.variables.insert(
                    name.clone(),
                    VarType::Struct {
                        struct_name: sname.clone(),
                        offset: slot,
                    },
                );

                for (i, arg) in cargs.iter().enumerate() {
                    if let Some(fname) = sfields.get(i) {
                        let is_flt = is_float_expr(arg, &self.ctx.variables);
                        let is_str = is_string_expr(arg, &self.ctx.variables);
                        let is_arr = is_array_expr(arg, &self.ctx.variables);
                        let is_map = is_map_expr(arg, &self.ctx.variables);
                        let field_key = format!("{}.{}", name, fname);
                        // Mixed literal kinds for this field: skip global
                        // markers; per-variable field_key stays precise.
                        let sf_mixed =
                            struct_field_markers_mixed_vars(&self.ctx.variables, &sname, fname);
                        if is_str {
                            self.ctx
                                .variables
                                .insert(field_key, VarType::StringOffset(0));
                            if !sf_mixed {
                                self.ctx.variables.insert(
                                    format!("struct_field_str:{}.{}", sname, fname),
                                    VarType::StringOffset(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_str:{}", fname),
                                    VarType::StringOffset(0),
                                );
                            }
                        } else if is_flt {
                            self.ctx.variables.insert(field_key, VarType::Float(0));
                            if !sf_mixed {
                                self.ctx.variables.insert(
                                    format!("struct_field_flt:{}.{}", sname, fname),
                                    VarType::Float(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_flt:{}", fname),
                                    VarType::Float(0),
                                );
                            }
                        } else if is_arr {
                            self.ctx.variables.insert(field_key, VarType::Array(0));
                            if !sf_mixed {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}.{}", sname, fname),
                                    VarType::Array(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}", fname),
                                    VarType::Array(0),
                                );
                            }
                            if is_string_array(arg, &self.ctx.variables) && !sf_mixed {
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
                            if !sf_mixed {
                                self.ctx.variables.insert(
                                    format!("struct_field_map:{}.{}", sname, fname),
                                    VarType::Map(0),
                                );
                                self.ctx
                                    .variables
                                    .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                            }
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

                let is_enum_from_ann = if let Some(t) = type_ann {
                    let bare = t.rsplit("::").next().unwrap_or(t);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if self.ctx.enums.contains(t) {
                        Some(t.to_string())
                    } else if self.ctx.enums.contains(bare) {
                        Some(bare.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                let is_enum = is_enum_from_ann.or_else(|| match value {
                    Expr::Identifier(ident) => {
                        if let Some(VarType::StringLabel(ename)) =
                            self.ctx.variables.get(&format!("var_enum_type:{}", ident))
                        {
                            Some(ename.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

                let is_str = is_explicit_str || is_string_expr(value, &self.ctx.variables);
                let is_arr = is_explicit_arr || is_array_expr(value, &self.ctx.variables);
                let is_struct = if is_enum.is_some() {
                    None
                } else {
                    is_struct_from_ann.or_else(|| self.get_expr_struct_name(value))
                };
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
                        is_arr || is_map || is_struct.is_some() || self.is_heap_expression(value)
                    }
                    _ => false,
                };
                // `let v = FRESH()[k]`: hidden-slot temp-index so the temp
                // is tracked and freed (alya-lang/alya#79, alya-lang/alya#81).
                // Covers scalars (no retain needed) and heap targets (retained via is_alias_heap).
                let desugar_temp_index = match value {
                    Expr::Index { array, .. } => {
                        matches!(
                            array.as_ref(),
                            Expr::Call { .. }
                                | Expr::Array(_)
                                | Expr::Map(_)
                                | Expr::StructInit { .. }
                        ) && (matches!(
                            type_ann,
                            Some("int")
                                | Some("i64")
                                | Some("i32")
                                | Some("u64")
                                | Some("u32")
                                | Some("uint")
                                | Some("byte")
                                | Some("char")
                                | Some("bool")
                                | Some("float")
                                | Some("f64")
                                | Some("f32")
                        ) || is_number_expr(value, &self.ctx.variables)
                            || is_float_expr(value, &self.ctx.variables)
                            || is_alias_heap
                            || self.is_heap_expression(value))
                    }
                    _ => false,
                };
                if desugar_temp_index {
                    if let Expr::Index { array, index } = value {
                        self.gen_temp_index_value(array, index);
                    }
                } else {
                    self.generate_expression(value);
                }
                // B1: named stores outlive the wrapping ring buffer.
                if string_store_needs_dup(value, &self.ctx.variables) {
                    arch::emit_str_store(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }

                if is_alias_heap {
                    arch::emit_rc_retain(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }

                self.emit_let_rebind_release(old_heap_offset, is_flt);
                // Reuse a rebound slot when safe (issue #80); otherwise
                // allocate fresh. `slot` is the home offset from here on.
                let slot = self.let_home_slot(old_heap_offset, is_flt);

                if let Some(ename) = &is_enum {
                    self.ctx.variables.insert(
                        format!("var_enum_type:{}", name),
                        VarType::StringLabel(ename.clone()),
                    );
                }

                if let Some(sname) = is_struct {
                    self.ctx.variables.insert(
                        name.clone(),
                        VarType::Struct {
                            struct_name: sname,
                            offset: slot,
                        },
                    );
                } else if is_map {
                    self.ctx.variables.insert(name.clone(), VarType::Map(slot));
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
                            if is_float_expr(v, &self.ctx.variables) {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_field_flt:{}", field),
                                        VarType::Float(0),
                                    );
                                    self.ctx.variables.insert(
                                        format!("map_flt:{}.{}", name, field),
                                        VarType::Float(0),
                                    );
                                }
                            }
                        }
                        // Whole-map veto (alya-lang/alya#39): any non-string
                        // value demotes for-loop values to Number (runtime
                        // dispatch in `say`); mirrors generate_assign below.
                        if entries
                            .iter()
                            .any(|(_, v)| !is_string_expr(v, &self.ctx.variables))
                        {
                            self.ctx
                                .variables
                                .insert(format!("map_nonstr:{}", name), VarType::Number(0));
                        }
                    }
                } else if is_arr {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Array(slot));
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
                        .insert(name.clone(), VarType::StringOffset(slot));
                } else if is_flt {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Float(self.ctx.stack_offset));
                } else if is_null {
                    self.ctx.variables.insert(name.clone(), VarType::Null(slot));
                } else {
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::Number(slot));
                    if matches!(
                        type_ann,
                        Some("int")
                            | Some("i64")
                            | Some("i32")
                            | Some("u64")
                            | Some("u32")
                            | Some("uint")
                            | Some("byte")
                            | Some("char")
                            | Some("bool")
                    ) || matches!(value, Expr::Number(_))
                        || is_number_expr(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", name), VarType::Number(0));
                    }
                    // NOTE: no clearing here (interim): clearing a stale
                    // int fact makes later `push(name)` retain a dynamic
                    // int and fault; proper fix is local tag spill so
                    // dynamic locals guard their retain at runtime.
                    // Call-bound marker for `say`: `let v = F(...)` with a
                    // dynamically-typed call result needs guarded print
                    // (null/map dispatch), while other Number dynamics
                    // (e.g. int loop vars, where 0 must print as 0 not
                    // null) keep the string/int path.
                    if matches!(value, Expr::Call { .. }) {
                        self.ctx
                            .variables
                            .insert(format!("call_bound:{}", name), VarType::Number(0));
                    } else {
                        self.ctx.variables.remove(&format!("call_bound:{}", name));
                    }
                    // Unsigned marker for u64-family annotations (B4).
                    if matches!(
                        type_ann,
                        Some("u64") | Some("u32") | Some("uint") | Some("usize")
                    ) || is_unsigned_expr(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_uint:{}", name), VarType::Number(0));
                    }
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
                    let prefix_str = format!("tuple_elem_str:{}:", target_fn);
                    let prefix_flt = format!("tuple_elem_flt:{}:", target_fn);
                    let matching: Vec<(bool, String)> = self
                        .ctx
                        .variables
                        .keys()
                        .filter_map(|k| {
                            if let Some(idx_str) = k.strip_prefix(&prefix_str) {
                                Some((true, idx_str.to_string()))
                            } else {
                                k.strip_prefix(&prefix_flt)
                                    .map(|idx_str| (false, idx_str.to_string()))
                            }
                        })
                        .collect();
                    for (is_str, idx_str) in matching {
                        if is_str {
                            self.ctx.variables.insert(
                                format!("tuple_elem_str:{}:{}", name, idx_str),
                                VarType::StringOffset(0),
                            );
                        } else {
                            self.ctx.variables.insert(
                                format!("tuple_elem_flt:{}:{}", name, idx_str),
                                VarType::Float(0),
                            );
                        }
                    }
                }

                if let Some(t) = type_ann {
                    if t.starts_with("Channel[string]") || t.starts_with("Channel[str]") {
                        self.ctx
                            .variables
                            .insert(format!("channel_elem_str:{}", name), VarType::Number(0));
                    }
                }
                if let Expr::Call {
                    name: cname,
                    args: cargs,
                } = value
                {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if bare == "new" {
                        if let Some(Expr::Index { array, index }) = cargs.first() {
                            if let (Expr::Identifier(arr_id), Expr::Identifier(type_id)) =
                                (&**array, &**index)
                            {
                                if arr_id == "Channel" && (type_id == "string" || type_id == "str")
                                {
                                    self.ctx.variables.insert(
                                        format!("channel_elem_str:{}", name),
                                        VarType::Number(0),
                                    );
                                }
                            }
                        }
                    }
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
                    let prefix_flt1 = format!("fn_ret_tuple_flt:{}:", cname);
                    let prefix_flt2 = format!("fn_ret_tuple_flt:{}:", bare);
                    let matching_flt: Vec<(String, String)> = self
                        .ctx
                        .variables
                        .keys()
                        .filter(|k| k.starts_with(&prefix_flt1) || k.starts_with(&prefix_flt2))
                        .filter_map(|k| {
                            k.strip_prefix(&prefix_flt1)
                                .or_else(|| k.strip_prefix(&prefix_flt2))
                                .map(|idx| (name.clone(), idx.to_string()))
                        })
                        .collect();
                    for (arr_name, idx_str) in matching_flt {
                        self.ctx.variables.insert(
                            format!("tuple_elem_flt:{}:{}", arr_name, idx_str),
                            VarType::Float(0),
                        );
                    }
                    let prefix_struct1 = format!("fn_ret_tuple_struct:{}:", cname);
                    let prefix_struct2 = format!("fn_ret_tuple_struct:{}:", bare);
                    let matching_struct: Vec<(String, String, String)> = self
                        .ctx
                        .variables
                        .iter()
                        .filter(|(k, _)| {
                            k.starts_with(&prefix_struct1) || k.starts_with(&prefix_struct2)
                        })
                        .filter_map(|(k, v)| {
                            let idx = k
                                .strip_prefix(&prefix_struct1)
                                .or_else(|| k.strip_prefix(&prefix_struct2))?;
                            if let VarType::Struct { struct_name, .. } = v {
                                Some((name.clone(), idx.to_string(), struct_name.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();
                    for (arr_name, idx_str, st) in matching_struct {
                        self.ctx.variables.insert(
                            format!("tuple_elem_struct:{}:{}", arr_name, idx_str),
                            VarType::Struct {
                                struct_name: st,
                                offset: 0,
                            },
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
                        if is_float_expr(elem, &self.ctx.variables) {
                            self.ctx.variables.insert(
                                format!("tuple_elem_flt:{}:{}", name, i),
                                VarType::Float(0),
                            );
                        }
                        if let Some(st) = self.get_expr_struct_name(elem) {
                            self.ctx.variables.insert(
                                format!("tuple_elem_struct:{}:{}", name, i),
                                VarType::Struct {
                                    struct_name: st,
                                    offset: 0,
                                },
                            );
                        }
                    }
                }
                if let Some(ann) = type_ann {
                    let ann = ann.trim();
                    if ann.starts_with('(') && ann.ends_with(')') {
                        for (i, ty) in ann[1..ann.len() - 1].split(',').enumerate() {
                            let ty = ty.trim();
                            if ty == "string" || ty == "str" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_str:{}:{}", name, i),
                                    VarType::StringOffset(0),
                                );
                            } else if ty == "float" || ty == "f64" || ty == "f32" {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_flt:{}:{}", name, i),
                                    VarType::Float(0),
                                );
                            } else if self.ctx.structs.contains_key(ty) {
                                self.ctx.variables.insert(
                                    format!("tuple_elem_struct:{}:{}", name, i),
                                    VarType::Struct {
                                        struct_name: ty.to_string(),
                                        offset: 0,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Home slot for a `let` value: reuse a rebound heap slot when safe,
    /// else allocate fresh. Reusing keeps the gen-time slot identical to
    /// every run-time iteration of a loop, so the rebind release always
    /// targets the live value (issue #80). Skipped for float values
    /// (their store convention differs).
    /// Returns the slot offset now holding the value.
    fn let_home_slot(&mut self, old_offset: Option<i32>, is_float_value: bool) -> i32 {
        let reuse = match old_offset {
            Some(off) if off > 0 => !is_float_value,
            _ => false,
        };
        if let Some(off) = old_offset.filter(|_| reuse) {
            arch::emit_store_var(&mut self.output, self.arch, off, self.ctx.stack_offset);
            off
        } else {
            arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
            self.ctx.stack_offset
        }
    }

    /// Release the previous heap value when `let` rebinds an existing
    /// name, then null the old slot. Nulling makes scope restores safe:
    /// a resurrected binding can only ever release null, never a stale
    /// pointer (double-free guard, issue #80). The new value is already
    /// generated (and retained when aliased), so dropping the old slot
    /// mirrors `=` exactly. No-op for first declarations (alya-lang/alya#79).
    fn emit_let_rebind_release(&mut self, old_offset: Option<i32>, is_flt: bool) {
        if let Some(old_offset) = old_offset {
            let temp_offset = self.temp_offset();
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                old_offset,
                self.ctx.stack_offset,
                self.os,
            );
            arch::emit_load_num(&mut self.output, self.arch, 0);
            arch::emit_store_var(
                &mut self.output,
                self.arch,
                old_offset,
                self.ctx.stack_offset,
            );
            self.ctx.stack_offset -= temp_offset;
            arch::emit_pop_temp(&mut self.output, self.arch);
            if is_flt {
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq %rax, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    fmov d0, x0\n");
                    }
                }
            }
        }
    }

    /// `let v = FRESH()[k]` / `v = FRESH()[k]` where the target is
    /// provably non-heap (int/float): evaluate the fresh container
    /// into a hidden tracked slot so index dispatch sees a map/array.
    /// The slot stays tracked (no immediate drop): scope machinery
    /// frees the last temp, and a mid-expression release would clobber
    /// the index result sitting in the return registers. Skipped
    /// (residual leak) for heap/dynamic results and non-fresh calls
    /// (alya-lang/alya#79).
    fn gen_temp_index_value(&mut self, array: &Expr, index: &Expr) -> bool {
        // Calls qualify only with proven freshness (or as direct struct
        // constructors): the temp is dropped after the read, which is
        // only sound for owned-separate values (alya-lang/alya#79).
        if let Expr::Call { name, .. } = array {
            if !(self.ctx.structs.contains_key(name)
                || crate::codegen::analysis::call_returns_fresh_value(name, &self.ctx.variables))
            {
                return false;
            }
        }
        let want_map = is_map_expr(array, &self.ctx.variables);
        let want_arr = !want_map && is_array_expr(array, &self.ctx.variables);
        if !(want_map || want_arr) {
            return false;
        }
        let hid = format!(
            "__idx_tmp_{}",
            self.ctx
                .next_label()
                .trim_start_matches('.')
                .trim_start_matches('L')
        );
        self.generate_expression(array);
        arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
        let off = self.ctx.stack_offset;
        if want_map {
            self.ctx.variables.insert(hid.clone(), VarType::Map(off));
        } else {
            self.ctx.variables.insert(hid.clone(), VarType::Array(off));
        }
        let rewritten = Expr::Index {
            array: Box::new(Expr::Identifier(hid.clone())),
            index: Box::new(index.clone()),
        };
        self.generate_expression(&rewritten);
        // No immediate drop: the index result sits in the return
        // registers and a release call would clobber it. The tracked
        // slot is freed by the normal scope machinery (covering the
        // last temp; earlier loop iterations keep leaking as before).
        true
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
            Expr::Call { name, .. } => {
                old_heap_offset.is_some()
                    && !crate::codegen::analysis::call_returns_fresh_value(
                        name,
                        &self.ctx.variables,
                    )
            }
            _ => false,
        };

        // `v = FRESH()[k]`: evaluate through a hidden slot so the temp
        // is tracked and freed (alya-lang/alya#79, alya-lang/alya#81).
        let mut generated = false;
        if let Expr::Index { array, index } = value {
            let fresh = matches!(
                array.as_ref(),
                Expr::Call { .. } | Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. }
            );
            let target_scalar = matches!(
                self.ctx.variables.get(&name),
                Some(VarType::Number(_)) | Some(VarType::Float(_))
            ) || is_number_expr(value, &self.ctx.variables)
                || is_float_expr(value, &self.ctx.variables);
            let target_heap = is_alias_heap || self.is_heap_expression(value);
            if fresh && (target_scalar || target_heap) {
                generated = self.gen_temp_index_value(array, index);
            }
        }
        if !generated {
            self.generate_expression(value);
        }
        // B1: named stores outlive the wrapping ring buffer.
        if string_store_needs_dup(value, &self.ctx.variables) {
            arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }

        if is_alias_heap {
            arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }

        if let Some(old_offset) = old_heap_offset {
            let temp_offset = self.temp_offset();
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                old_offset,
                self.ctx.stack_offset,
                self.os,
            );
            self.ctx.stack_offset -= temp_offset;
            arch::emit_pop_temp(&mut self.output, self.arch);
            if is_flt {
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq %rax, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    fmov d0, x0\n");
                    }
                }
            }
        }

        let is_local_var = match self.ctx.variables.get(&name) {
            Some(VarType::Number(off))
            | Some(VarType::Float(off))
            | Some(VarType::StringOffset(off))
            | Some(VarType::Array(off))
            | Some(VarType::Map(off))
            | Some(VarType::Null(off))
            | Some(VarType::Struct { offset: off, .. }) => *off != 0,
            _ => false,
        };

        if !is_local_var {
            if let Some((symbol, _)) = self.ctx.globals.get(&name).cloned() {
                arch::emit_store_global(&mut self.output, self.arch, &symbol, self.os);
                return;
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
                | VarType::Struct { offset, .. }
                | VarType::Interface { offset, .. } => {
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
                            // Whole-map veto mirroring the inference pass
                            // (alya-lang/alya#39): gates the for-loop
                            // string aggregation in control.rs.
                            if entries
                                .iter()
                                .any(|(_, v)| !is_string_expr(v, &self.ctx.variables))
                            {
                                self.ctx
                                    .variables
                                    .insert(format!("map_nonstr:{}", name), VarType::Number(0));
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
                    } else if let VarType::Interface { interface_name, .. } = var_type {
                        self.ctx.variables.insert(
                            name.clone(),
                            VarType::Interface {
                                interface_name,
                                offset,
                            },
                        );
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
                            // Struct rebinding voids any stale int fact.
                            self.ctx.variables.remove(&format!("var_is_int:{}", name));
                        } else {
                            self.ctx
                                .variables
                                .insert(name.clone(), VarType::Number(offset));
                            // Maintain the exact int fact across `x = ...`
                            // (mirrors `let`): proven-int RHS keeps the
                            // marker so later `push(x)` skips `rc_retain`
                            // (which faults on large 8-aligned ints).
                            // NOTE (interim): no clearing of stale markers;
                            // clearing makes dynamic rebinds retain and
                            // fault; proper fix is local tag spill.
                            if matches!(value, Expr::Number(_))
                                || is_number_expr(value, &self.ctx.variables)
                            {
                                self.ctx
                                    .variables
                                    .insert(format!("var_is_int:{}", name), VarType::Number(0));
                            }
                            // Call-bound marker for `say` (see `let`).
                            if matches!(value, Expr::Call { .. }) {
                                self.ctx
                                    .variables
                                    .insert(format!("call_bound:{}", name), VarType::Number(0));
                            } else {
                                self.ctx.variables.remove(&format!("call_bound:{}", name));
                            }
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
            let prefix_flt1 = format!("fn_ret_tuple_flt:{}:", cname);
            let prefix_flt2 = format!("fn_ret_tuple_flt:{}:", bare);
            let matching_flt: Vec<(String, String)> = self
                .ctx
                .variables
                .keys()
                .filter(|k| k.starts_with(&prefix_flt1) || k.starts_with(&prefix_flt2))
                .filter_map(|k| {
                    k.strip_prefix(&prefix_flt1)
                        .or_else(|| k.strip_prefix(&prefix_flt2))
                        .map(|idx| (name.clone(), idx.to_string()))
                })
                .collect();
            for (arr_name, idx_str) in matching_flt {
                self.ctx.variables.insert(
                    format!("tuple_elem_flt:{}:{}", arr_name, idx_str),
                    VarType::Float(0),
                );
            }
            let prefix_struct1 = format!("fn_ret_tuple_struct:{}:", cname);
            let prefix_struct2 = format!("fn_ret_tuple_struct:{}:", bare);
            let matching_struct: Vec<(String, String, String)> = self
                .ctx
                .variables
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix_struct1) || k.starts_with(&prefix_struct2))
                .filter_map(|(k, v)| {
                    let idx = k
                        .strip_prefix(&prefix_struct1)
                        .or_else(|| k.strip_prefix(&prefix_struct2))?;
                    if let VarType::Struct { struct_name, .. } = v {
                        Some((name.clone(), idx.to_string(), struct_name.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            for (arr_name, idx_str, st) in matching_struct {
                self.ctx.variables.insert(
                    format!("tuple_elem_struct:{}:{}", arr_name, idx_str),
                    VarType::Struct {
                        struct_name: st,
                        offset: 0,
                    },
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
                if is_float_expr(elem, &self.ctx.variables) {
                    self.ctx
                        .variables
                        .insert(format!("tuple_elem_flt:{}:{}", name, i), VarType::Float(0));
                }
                if let Some(st) = self.get_expr_struct_name(elem) {
                    self.ctx.variables.insert(
                        format!("tuple_elem_struct:{}:{}", name, i),
                        VarType::Struct {
                            struct_name: st,
                            offset: 0,
                        },
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

        let is_weak = self.is_struct_field_weak(object, field);

        let temp_offset = self.temp_offset();
        self.generate_expression(object);
        // Null-base trap (alya-lang/alya#74): a struct field store through
        // a zero-word base segfaults. Trap it like `!` does, reporting a
        // catchable error instead. Optional-chaining forms (`?.`) are
        // excluded: null short-circuits there by design. Any zero word
        // traps — only heap objects are valid bases, so integers, floats
        // and null share the same (previously faulting) path.
        if !matches!(
            object,
            Expr::OptionalFieldAccess { .. } | Expr::OptionalIndex { .. }
        ) {
            arch::emit_cmp_imm(&mut self.output, self.arch, 0);
            arch::emit_cond_jump(
                &mut self.output,
                self.arch,
                BinaryOp::Equal,
                false,
                "alya_error_null_field",
                false,
            );
        }
        arch::emit_push_temp(&mut self.output, self.arch);
        self.ctx.stack_offset += temp_offset;

        self.generate_expression(value);
        if !is_weak && self.is_heap_expression(value) {
            arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }
        // B1: named stores outlive the wrapping ring buffer.
        if string_store_needs_dup(value, &self.ctx.variables) {
            arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }
        self.ctx.stack_offset -= temp_offset;
        let field_idx = self
            .resolve_struct_field_target(object, field)
            .map(|(_, idx)| idx)
            .unwrap_or(0);
        arch::emit_struct_field_set(&mut self.output, self.arch, field_idx);
    }

    pub(super) fn generate_index_assign(&mut self, array: &Expr, index: &Expr, value: &Expr) {
        if let Some(sname) = self.get_expr_struct_name(array) {
            let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
            let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
            let cand1 = format!("{}__{}", sname, "operator[]=");
            let cand2 = format!("{}__{}", bare_sname, "operator[]=");
            let matched = if self.ctx.functions.contains(&cand1) {
                Some(cand1)
            } else if self.ctx.functions.contains(&cand2) {
                Some(cand2)
            } else {
                self.ctx
                    .functions
                    .iter()
                    .find(|f| f.ends_with("__operator[]="))
                    .cloned()
            };
            if let Some(call_name) = matched {
                self.generate_expression(&Expr::Call {
                    name: call_name,
                    args: vec![array.clone(), index.clone(), value.clone()],
                });
                return;
            }
        }

        if is_map_expr(array, &self.ctx.variables)
            || is_string_expr(index, &self.ctx.variables)
            || matches!(index, Expr::String(_))
        {
            let actual_args = [array, index, value];
            for arg in actual_args.iter() {
                self.generate_expression(arg);
                if std::ptr::eq(*arg, value) && self.store_value_needs_retain(value) {
                    if is_tag_carrying_read(value, &self.ctx.variables) {
                        // The value tag is fresh: skip the retain for
                        // int/float scalars (retaining a large 8-aligned
                        // int faults), keep it for heap/unknown kinds.
                        self.emit_tag_guarded_retain();
                    } else {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                }
                // B1: named stores outlive the wrapping ring buffer.
                if std::ptr::eq(*arg, value) && string_store_needs_dup(value, &self.ctx.variables) {
                    arch::emit_str_store(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
                arch::emit_push_temp(&mut self.output, self.arch);
            }
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "set",
                3,
                self.ctx.stack_offset,
                self.os,
            );
            // Record a value-kind tag for literal writes so variable-key
            // reads can dispatch on it. Only fully static shapes qualify
            // (identifier map, literal key, literal value): re-evaluating
            // anything else could duplicate side effects. Dynamics leave
            // the tag cleared by `set` itself (unknown = 0).
            if let (Expr::Identifier(_), Expr::String(_)) = (array, index) {
                let kind = crate::codegen::analysis::value_kind_tag(value, &self.ctx.variables);
                if kind != crate::codegen::kinds::KIND_UNKNOWN {
                    let tag_args = [array.clone(), index.clone(), Expr::Number(kind as i128)];
                    for arg in tag_args.iter() {
                        self.generate_expression(arg);
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }
                    arch::emit_function_call(
                        &mut self.output,
                        self.arch,
                        "map_set_tag",
                        3,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
            }
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
            } else if let (Expr::Identifier(map_name), Expr::String(_)) = (array, index) {
                // Non-string write voids the whole-map string claim
                // (alya-lang/alya#39); mirrors the inference veto.
                self.ctx
                    .variables
                    .insert(format!("map_nonstr:{}", map_name), VarType::Number(0));
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
            if is_float_expr(value, &self.ctx.variables) {
                if let Expr::String(field) = index {
                    self.ctx
                        .variables
                        .insert(format!("map_field_flt:{}", field), VarType::Float(0));
                }
                if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                    self.ctx
                        .variables
                        .insert(format!("map_flt:{}.{}", map_name, field), VarType::Float(0));
                }
            }
            // Literal writes carry ground truth about the stored kind:
            // drop per-key markers of contradicting families left over
            // from earlier writes (a stale `map_str` marker miscompiles a
            // later read, e.g. `%s` on an int segfaults). Dynamic values
            // leave markers untouched.
            // Kinds: 0 = string, 1 = float, 2 = map, 3 = other.
            if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                let lit_kind: Option<u8> = match value {
                    Expr::String(_) => Some(0),
                    Expr::Float(_) => Some(1),
                    Expr::Number(_) => Some(3),
                    Expr::Map(_) => Some(2),
                    Expr::Array(_) | Expr::Null | Expr::StructInit { .. } => Some(3),
                    _ => None,
                };
                if let Some(kind) = lit_kind {
                    let mut drop_keys = Vec::new();
                    if kind != 0 {
                        drop_keys.push(format!("map_str:{}.{}", map_name, field));
                        drop_keys.push(format!("map_field_str:{}", field));
                    }
                    if kind != 1 {
                        drop_keys.push(format!("map_flt:{}.{}", map_name, field));
                        drop_keys.push(format!("map_field_flt:{}", field));
                    }
                    if kind != 2 {
                        drop_keys.push(format!("map_map:{}.{}", map_name, field));
                        drop_keys.push(format!("map_field_map:{}", field));
                    }
                    for k in drop_keys {
                        self.ctx.variables.remove(&k);
                    }
                }
            }
        } else {
            let temp_offset = self.temp_offset();
            self.generate_expression(array);
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;

            self.generate_expression(index);
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;

            self.generate_expression(value);
            if self.store_value_needs_retain(value) {
                arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
            // B1: named stores outlive the wrapping ring buffer.
            if string_store_needs_dup(value, &self.ctx.variables) {
                arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
            self.ctx.stack_offset -= temp_offset * 2;
            let set_kind = value_kind_tag(value, &self.ctx.variables);
            arch::emit_array_set(&mut self.output, self.arch, set_kind);
        }
    }
}
