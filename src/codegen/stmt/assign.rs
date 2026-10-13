use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::nested::CLOSURE_NEW;
use crate::codegen::analysis::{
    escape_string, expr_mentions_var, find_operator_overload, is_array_expr, is_array_fold_true,
    is_definitely_not_numeric, is_float_array, is_float_expr, is_map_entry_int, is_map_expr,
    is_null_expr, is_number_expr, is_proven_int_expr, is_string_array, is_string_expr,
    is_tag_carrying_read, is_unsigned_expr, string_store_needs_dup,
    struct_field_markers_mixed_vars, typeof_operand_is_repeatable, value_kind_tag,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

impl CodeGen {
    /// Clears per-key map value-struct claims for `name`
    /// (alya-lang/alya#163): any rebind or element write may change
    /// what each key holds, and a stale claim would misdispatch a
    /// later method call to the wrong struct's function.
    pub(super) fn clear_map_struct_markers(&mut self, name: &str) {
        let prefix = format!("map_struct:{}:", name);
        let stale: Vec<String> = self
            .ctx
            .variables
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        for key in stale {
            self.ctx.variables.remove(&key);
        }
    }

    /// Seeds per-key value-struct claims from a map literal
    /// (alya-lang/alya#163): string-literal keys whose value has a
    /// proven struct publish `map_struct:{var}:{key}`, consulted by
    /// `get_expr_struct_name` so method calls on map-retrieved
    /// receivers bind the construction type exactly like the
    /// checker's first-pair inference. Only proven structs seed;
    /// everything else stays dynamic. Deliberately per-variable
    /// (no bare `map_field_struct` form): a global claim would let
    /// one map's key govern unrelated maps' reads.
    pub(super) fn seed_map_struct_markers(
        &mut self,
        name: &str,
        entries: &[(crate::ast::Expr, crate::ast::Expr)],
    ) {
        for (k, v) in entries {
            if let crate::ast::Expr::String(field) = k {
                if let Some(sname) = self.get_expr_struct_name(v) {
                    self.ctx.variables.insert(
                        format!("map_struct:{}:{}", name, field),
                        VarType::Struct {
                            struct_name: sname,
                            offset: 0,
                        },
                    );
                }
            }
        }
    }

    pub(super) fn generate_let(&mut self, name: &str, type_ann: Option<&str>, value: &Expr) {
        let name = name.to_string();
        // alya-lang/alya#163: a rebind voids per-key value-struct
        // claims (a stale claim would misdispatch a later method
        // call); map literals re-seed below.
        self.clear_map_struct_markers(&name);
        // alya-lang/alya#171: a rebind voids any spilled kind tag (a
        // stale tag would misread the new value: 0 shares null's zero
        // word). Tag-carrying RHS re-spills below.
        self.ctx.variables.remove(&format!("var_tag_slot:{}", name));
        // alya-lang/alya#100: track closure-valued locals for container
        // cascades (`value_kind_tag` reads this). Never cleared: stale
        // claims are safe (release gates skip non-heap; heap values
        // release correctly under kind 6 either way).
        // alya-lang/alya#183: `was_owned` snapshots pre-statement
        // ownership for the old-value release below (same rationale
        // as in `generate_assign`).
        let previously_owned = self
            .ctx
            .variables
            .contains_key(&format!("closure_owned:{}", name));
        if matches!(value, Expr::Call { name: c, .. } if c == CLOSURE_NEW)
            || matches!(value, Expr::Identifier(n) if self.ctx.variables.contains_key(&format!("closure_val:{}", n)))
        {
            self.ctx
                .variables
                .insert(format!("closure_val:{}", name), VarType::Number(0));
            // alya-lang/alya#183: direct materializations transfer
            // their creation share to this slot, and copies just
            // retained one above (`is_alias_heap`); either way the
            // slot owns a share that scope cleanup must drop. Never
            // cleared either: a stale marker on a rebound slot
            // releases a non-heap value, which the probed release
            // skips, and offsets dedup against typed arms.
            self.ctx
                .variables
                .insert(format!("closure_owned:{}", name), VarType::Number(0));
        }
        // alya-lang/alya#100: captured locals live in cells (no stack
        // slot, no `variables` entry: absence routes reads).
        if let Some(loc) = self.capture_loc(&name) {
            self.emit_cap_assign(&name, value, loc);
            return;
        }
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
                if self.value_needs_heap_retain(value) {
                    // Retain when maybe-heap; direct only when proven.
                    if self.value_proven_heap(value) {
                        arch::emit_rc_retain_direct(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
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
                    if let Expr::String(s) = value {
                        self.ctx.variables.insert(
                            format!("var_str_val:{}", name),
                            VarType::StringLabel(s.clone()),
                        );
                    }
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
                        || is_proven_int_expr(value, &self.ctx.variables)
                        || is_map_entry_int(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", name), VarType::Number(0));
                    }
                    // #151: conversion fast paths need exact int-ness, but
                    // `var_is_int` stays stale-kept by design (retain
                    // consumers). Track rebind-risk separately: a
                    // non-proven RHS may hold anything at runtime, so later
                    // conversions must take the heuristic path. Proven RHS
                    // clears (lets self-heal, e.g. shadowing).
                    if is_proven_int_expr(value, &self.ctx.variables)
                        || is_map_entry_int(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .remove(&format!("var_rebound_nonint:{}", name));
                    } else {
                        self.ctx
                            .variables
                            .insert(format!("var_rebound_nonint:{}", name), VarType::Number(0));
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
        // Closure-owned slots join through their marker, gated on
        // pre-statement ownership (alya-lang/alya#183); the probed
        // release skips a stale marker over a non-heap rebound value.
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
            Some(VarType::Number(off))
            | Some(VarType::Float(off))
            | Some(VarType::StringOffset(off))
            | Some(VarType::Null(off))
                if previously_owned =>
            {
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
                self.emit_let_rebind_release(&name, old_heap_offset, false);
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
                self.emit_let_rebind_release(&name, old_heap_offset, false);
                let slot = self.let_home_slot(old_heap_offset, false);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::StringOffset(slot));
                self.ctx.variables.insert(
                    format!("var_str_val:{}", name),
                    VarType::StringLabel(s.clone()),
                );
            }
            Expr::Array(elements) => {
                let is_str_arr = (!elements.is_empty()
                    && elements
                        .iter()
                        .all(|e| is_string_expr(e, &self.ctx.variables)))
                    || matches!(type_ann, Some("string[]") | Some("str[]"));
                // Mixed literals are NOT float arrays (#95): require all
                // elements float, mirroring is_str_arr's `all` above.
                // First-only marking misread `[1.5, 1][1]` as f64 bits.
                // Explicit annotations promise conversion-on-read, so
                // they also plant `arr_flt_ann` (push vetoes skip those).
                let ann_flt_arr = matches!(type_ann, Some("float[]") | Some("f64[]"));
                let is_flt_arr = (!elements.is_empty()
                    && elements
                        .iter()
                        .all(|e| is_float_expr(e, &self.ctx.variables)))
                    || ann_flt_arr;
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

                self.emit_let_rebind_release(&name, old_heap_offset, false);
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
                if ann_flt_arr {
                    self.ctx
                        .variables
                        .insert(format!("arr_flt_ann:{}", name), VarType::Number(0));
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

                self.emit_let_rebind_release(&name, old_heap_offset, false);
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

                self.emit_let_rebind_release(&name, old_heap_offset, false);
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
                    // Maybe-heap (old typed-local condition plus
                    // union-proven untyped locals); the emission below
                    // goes direct only when proven.
                    Expr::Identifier(_) => self.value_needs_heap_retain(value),
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
                // alya-lang/alya#170: preserve dynamic kind tag in a local spill slot across let-bindings
                // so 0 is not confused with null at runtime.
                let val_carries_tag = matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                    && is_tag_carrying_read(value, &self.ctx.variables)
                    && !is_proven_int_expr(value, &self.ctx.variables)
                    && !is_null_expr(value, &self.ctx.variables);
                // NOTE (alya-lang/alya#171): may-fact kind claims
                // (`fn_ret_map`/`fn_ret_str`/..., e.g. a map-typed
                // argument flowing to an untyped return) must not veto
                // the spill: the tag is exact while may-facts are
                // existential, and every tag consumer falls back
                // safely on it.
                // The spill below runs after retain/store calls (all
                // clobber the tag register, e.g. nested-map reads keep
                // their alias retain), so preserve the fresh tag across
                // them exactly like the assignment twin below.
                if val_carries_tag {
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    push %rdx\n");
                            self.ctx.stack_offset += 8;
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    str x1, [sp, #-16]!\n");
                            self.ctx.stack_offset += 16;
                        }
                    }
                }
                // Implicit `: float` conversion for unknown-kind values
                // (#83): the slot below is tracked Float, but an int
                // arriving here keeps int bits and reads back as a
                // denormal. Proven-float values no-op inside.
                if is_explicit_flt {
                    self.emit_implicit_float_convert(value);
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
                    // Maybe-heap alias: retain; direct only when proven
                    // (union or fresh value — never a stale typed slot).
                    if self.value_proven_heap(value) {
                        arch::emit_rc_retain_direct(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                }

                self.emit_let_rebind_release(&name, old_heap_offset, is_flt);
                // Reuse a rebound slot when safe (issue #80); otherwise
                // allocate fresh. `slot` is the home offset from here on.
                // alya-lang/alya#171: pop the tag pushed above BEFORE the
                // home allocation (a fresh home pushes the value; popping
                // after would take the value, not the tag). The tag stays
                // live in %edx/w1 across the pure home/type bookkeeping
                // below (no calls there); it is spilled after the type
                // chain so `stack_offset`-reading homes (float) keep
                // working.
                if val_carries_tag {
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    pop %rdx\n");
                            self.ctx.stack_offset -= 8;
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    ldr x1, [sp], #16\n");
                            self.ctx.stack_offset -= 16;
                        }
                    }
                }
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
                        self.seed_map_struct_markers(&name, entries);
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
                            if is_null_expr(v, &self.ctx.variables) {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_field_null:{}", field),
                                        VarType::Null(0),
                                    );
                                    self.ctx.variables.insert(
                                        format!("map_null:{}.{}", name, field),
                                        VarType::Null(0),
                                    );
                                }
                            }
                            if is_proven_int_expr(v, &self.ctx.variables)
                                || matches!(v, Expr::Number(_))
                            {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_field_int:{}", field),
                                        VarType::Number(0),
                                    );
                                    self.ctx.variables.insert(
                                        format!("map_int:{}.{}", name, field),
                                        VarType::Number(0),
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
                        // Key-level vetoes (alya-lang/alya#138): mirror the
                        // inference negatives so bare `map_field_str` claims
                        // cannot govern reads of these slots.
                        for (k, v) in entries {
                            if !is_string_expr(v, &self.ctx.variables) {
                                if let Expr::String(field) = k {
                                    self.ctx.variables.insert(
                                        format!("map_nonstr_key:{}", field),
                                        VarType::Number(0),
                                    );
                                }
                            }
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
                    if matches!(type_ann, Some("float[]") | Some("f64[]")) {
                        self.ctx
                            .variables
                            .insert(format!("arr_flt_ann:{}", name), VarType::Number(0));
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
                    if val_carries_tag {
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        let tag_slot = self.ctx.stack_offset;
                        match self.arch {
                            Architecture::X64 => {
                                self.output
                                    .push_str(&format!("    movl %edx, -{}(%rbp)\n", tag_slot));
                            }
                            Architecture::ARM64 => {
                                crate::codegen::arch::arm64::loads::emit_arm64_store_x29_offset(
                                    &mut self.output,
                                    "w1",
                                    tag_slot,
                                    "x9",
                                );
                            }
                        }
                        self.ctx
                            .variables
                            .insert(format!("var_tag_slot:{}", name), VarType::Number(tag_slot));
                    }
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
                        || is_proven_int_expr(value, &self.ctx.variables)
                        || is_map_entry_int(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .insert(format!("var_is_int:{}", name), VarType::Number(0));
                    }
                    // #151: conversion fast paths need exact int-ness (see
                    // the top-level `let` twin above): a non-proven RHS may
                    // hold anything at runtime.
                    if is_proven_int_expr(value, &self.ctx.variables)
                        || is_map_entry_int(value, &self.ctx.variables)
                    {
                        self.ctx
                            .variables
                            .remove(&format!("var_rebound_nonint:{}", name));
                    } else {
                        self.ctx
                            .variables
                            .insert(format!("var_rebound_nonint:{}", name), VarType::Number(0));
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

                // alya-lang/alya#171: spill the exact runtime tag (still
                // live in %edx/w1 from the pop above; only pure
                // bookkeeping intervened) for any home kind. Runs after
                // the type chain so `stack_offset`-reading homes (float)
                // observe pristine state. Pinned memo slots (allocated at
                // function entry) are reused without new allocs so every
                // branch/iteration shares one offset; fresh mid-body allocs
                // stay out of the memo (their offsets diverge per branch).
                if val_carries_tag {
                    let tag_slot = match self.ctx.tag_slot_memo.get(&name).cloned() {
                        Some(off) => off,
                        None => {
                            arch::emit_allocate_var(
                                &mut self.output,
                                self.arch,
                                &mut self.ctx.stack_offset,
                            );
                            self.ctx.stack_offset
                        }
                    };
                    match self.arch {
                        Architecture::X64 => {
                            self.output
                                .push_str(&format!("    movl %edx, -{}(%rbp)\n", tag_slot));
                        }
                        Architecture::ARM64 => {
                            crate::codegen::arch::arm64::loads::emit_arm64_store_x29_offset(
                                &mut self.output,
                                "w1",
                                tag_slot,
                                "x9",
                            );
                        }
                    }
                    self.ctx
                        .variables
                        .insert(format!("var_tag_slot:{}", name), VarType::Number(tag_slot));
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
                        // #101: qualified-spelled references consult exact
                        // markers only.
                        let vt = self.ctx.variables.get(&k1).or_else(|| {
                            if crate::codegen::analysis::is_simple_name(target_fn) {
                                self.ctx.variables.get(&k2)
                            } else {
                                None
                            }
                        });
                        if let Some(vt) = vt.cloned() {
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
    /// Probe-free direct release only for union-proven names (every
    /// write is null-or-heap, so the old value is too); all other
    /// slots keep the probed call.
    fn emit_let_rebind_release(&mut self, name: &str, old_offset: Option<i32>, is_flt: bool) {
        if let Some(old_offset) = old_offset {
            let temp_offset = self.temp_offset();
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;
            // Direct only for union-proven names (see slot_proven_heap).
            if self.slot_proven_heap(name) {
                arch::emit_rc_release_stack_direct(
                    &mut self.output,
                    self.arch,
                    old_offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                arch::emit_rc_release_stack(
                    &mut self.output,
                    self.arch,
                    old_offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            }
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

    /// Captured-cell write (alya-lang/alya#100): `name` lives in the
    /// cell at `loc`. Mirrors the index-assign discipline (generate,
    /// retain-new, dup strings, release-old, store value+kind). No
    /// `variables` entry is recorded: absence routes reads through the
    /// cell.
    pub(super) fn emit_cap_assign(&mut self, name: &str, value: &Expr, loc: super::super::CapLoc) {
        // Resolve the owner array up front (stable, retained): stores
        // address the holder by slot.
        let owner_off = self.emit_cap_owner(loc);
        self.generate_expression(value);
        if self.store_value_needs_retain(value) {
            if self.value_proven_heap(value) {
                arch::emit_rc_retain_direct(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else if is_tag_carrying_read(value, &self.ctx.variables) {
                self.emit_tag_guarded_retain();
            } else {
                arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
        }
        if string_store_needs_dup(value, &self.ctx.variables) {
            arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }
        let temp: i32 = match self.arch {
            Architecture::ARM64 => 16,
            _ => 8,
        };
        arch::emit_push_temp(&mut self.output, self.arch);
        self.ctx.stack_offset += temp;
        // Read-old through the resolved owner slot (single hop: the
        // owner array is already home).
        arch::emit_load_var(
            &mut self.output,
            self.arch,
            owner_off,
            self.ctx.stack_offset,
        );
        arch::emit_push_temp(&mut self.output, self.arch);
        self.ctx.stack_offset += temp;
        arch::emit_load_num(&mut self.output, self.arch, loc.vi as i64);
        arch::emit_array_get(&mut self.output, self.arch);
        self.ctx.stack_offset -= temp;
        arch::emit_rc_release(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        arch::emit_pop_temp(&mut self.output, self.arch);
        self.ctx.stack_offset -= temp;
        // Kind is static-proven or UNKNOWN (array-element discipline).
        // One correction: `+` on dynamic operands may concatenate
        // (string result) while arithmetic claims int — without the
        // operand markers cells never carry, readers would misdispatch.
        // Downgrade such ints to UNKNOWN unless both sides are proven
        // (classify-then-print handles both outcomes exactly).
        let mut kind = value_kind_tag(value, &self.ctx.variables);
        if kind == crate::codegen::kinds::KIND_INT {
            if let Expr::Binary {
                left,
                op: crate::ast::BinaryOp::Add,
                right,
            } = value
            {
                if !(is_proven_int_expr(left, &self.ctx.variables)
                    && is_proven_int_expr(right, &self.ctx.variables))
                {
                    kind = crate::codegen::kinds::KIND_UNKNOWN;
                }
            }
        }
        match self.arch {
            Architecture::X64 => {
                self.output.push_str(&format!("    movl ${}, %edx\n", kind));
            }
            Architecture::ARM64 => {
                self.output.push_str(&format!("    mov w1, #{}\n", kind));
            }
        }
        arch::emit_cap_store(&mut self.output, self.arch, owner_off, loc.vi);
        // Free the resolved-owner temp (two-hop readers only): it
        // sits below every later spill, so leaking it shifts
        // (%rsp)-relative consumers exactly like the callee-temp
        // leak fixed at the captured-callee dispatch site.
        if loc.chain_pos.is_some() {
            match self.arch {
                Architecture::X64 => {
                    self.output.push_str("    add $8, %rsp\n");
                }
                Architecture::ARM64 => {
                    self.output.push_str("    add sp, sp, #16\n");
                }
            }
            self.ctx.stack_offset -= temp;
        }
        // Closure-ness marker for `value_kind_tag` (container cascades;
        // never cleared: stale claims are release-safe, see `let`).
        if kind == crate::codegen::kinds::KIND_STRUCT {
            self.ctx
                .variables
                .insert(format!("closure_val:{}", name), VarType::Number(0));
        }
    }

    pub(super) fn generate_assign(&mut self, name: &str, value: &Expr) {
        let name = name.to_string();
        // alya-lang/alya#163: rebinds void per-key value-struct claims.
        self.clear_map_struct_markers(&name);
        // alya-lang/alya#171: a rebind voids any spilled kind tag (a
        // stale tag would misread the new value: 0 shares null's zero
        // word). Tag-carrying RHS re-spills below.
        self.ctx.variables.remove(&format!("var_tag_slot:{}", name));
        // alya-lang/alya#100: closure-valued locals track for cascades
        // (never cleared: stale claims are release-safe; see `let`).
        // alya-lang/alya#183: ownership twin of the line above — the
        // assign path retains closure copies exactly like `let`, so
        // the slot owns a share scope cleanup must drop. `was_owned`
        // below snapshots pre-statement ownership: the old-value
        // release must not see the marker this statement just added,
        // or rebinding a borrowed slot (e.g. a param) would drop a
        // share the borrower still owns.
        let previously_owned = self
            .ctx
            .variables
            .contains_key(&format!("closure_owned:{}", name));
        if matches!(value, Expr::Call { name: c, .. } if c == CLOSURE_NEW)
            || matches!(value, Expr::Identifier(n) if self.ctx.variables.contains_key(&format!("closure_val:{}", n)))
        {
            self.ctx
                .variables
                .insert(format!("closure_val:{}", name), VarType::Number(0));
            self.ctx
                .variables
                .insert(format!("closure_owned:{}", name), VarType::Number(0));
        }
        // alya-lang/alya#100: captured writes route via cells.
        if let Some(loc) = self.capture_loc(&name) {
            self.emit_cap_assign(&name, value, loc);
            return;
        }
        // Target-aware floatness (#83): assigning an unknown-kind value
        // into an existing float slot must convert AND track float, or
        // the slot keeps int bits while every later read treats them as
        // a double. Value-proven floats behave as before.
        let target_is_flt = matches!(self.ctx.variables.get(&name), Some(VarType::Float(_)));
        let is_flt = target_is_flt || is_float_expr(value, &self.ctx.variables);
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
            // alya-lang/alya#183: closure-owned slots (see `let`
            // above), gated on pre-statement ownership so a fresh
            // marker never drops a borrowed old value; stale markers
            // over rebound non-heap values are skipped by the probed
            // release.
            Some(VarType::Number(off))
            | Some(VarType::Float(off))
            | Some(VarType::StringOffset(off))
            | Some(VarType::Null(off))
                if previously_owned =>
            {
                if *off > 0 {
                    Some(*off)
                } else {
                    None
                }
            }
            _ => None,
        };
        let is_alias_heap = match value {
            // Maybe-heap (old conditions plus union-proven untyped
            // locals); the emission below goes direct only when proven.
            Expr::Identifier(_) => self.value_needs_heap_retain(value),
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
        // alya-lang/alya#171: reassignments from tag-carrying reads
        // (e.g. jsonpath `curr = curr[tok]`) keep the runtime kind tag
        // across the retain/store calls below (all clobber the tag
        // register), mirroring the let-bound spill. Without it a later
        // `is null`/`== null` misreads 0 as null. May-fact kind claims
        // never veto the spill: the tag is exact (see let twin above).
        let assign_carries_tag = matches!(self.arch, Architecture::X64 | Architecture::ARM64)
            && is_tag_carrying_read(value, &self.ctx.variables)
            && !is_proven_int_expr(value, &self.ctx.variables)
            && !is_null_expr(value, &self.ctx.variables);
        if assign_carries_tag {
            match self.arch {
                Architecture::X64 => {
                    self.output.push_str("    push %rdx\n");
                    self.ctx.stack_offset += 8;
                }
                Architecture::ARM64 => {
                    self.output.push_str("    str x1, [sp, #-16]!\n");
                    self.ctx.stack_offset += 16;
                }
            }
        }
        // Implicit conversion into an existing float slot (#83): the
        // slot reads back as float, but an unknown-kind int value keeps
        // int bits (later read as a denormal). Proven-float values no-op
        // inside the converter. The rax->xmm0/d0 sync follows for
        // index/ternary/call values, which live rax-only (the float
        // store below reads the float register): without it a
        // statically-float index stores stale xmm0 contents.
        if target_is_flt
            && !is_float_expr(value, &self.ctx.variables)
            && !is_definitely_not_numeric(value, &self.ctx.variables)
        {
            self.emit_implicit_float_convert(value);
        }
        if target_is_flt
            && matches!(
                value,
                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
            )
        {
            self.emit_sync_float_reg();
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

        // B1: named stores outlive the wrapping ring buffer.
        // Self-accumulating local string stores (`res += ...`, `res = res + ...`)
        // are transient loop accumulators that live in the wrapping ring buffer.
        // Duplicating every intermediate step into the immortal stable region
        // fills it monotonically and overflows on long loops (issue #116).
        let is_self_accum = is_local_var && expr_mentions_var(value, &name);
        if !is_self_accum && string_store_needs_dup(value, &self.ctx.variables) {
            arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }

        if is_alias_heap {
            // Maybe-heap alias: retain; direct only when proven
            // (union or fresh value — never a stale typed slot).
            if self.value_proven_heap(value) {
                arch::emit_rc_retain_direct(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
        }

        if let Some(old_offset) = old_heap_offset {
            let temp_offset = self.temp_offset();
            arch::emit_push_temp(&mut self.output, self.arch);
            self.ctx.stack_offset += temp_offset;
            // Direct only for union-proven names (see slot_proven_heap).
            if self.slot_proven_heap(&name) {
                arch::emit_rc_release_stack_direct(
                    &mut self.output,
                    self.arch,
                    old_offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                arch::emit_rc_release_stack(
                    &mut self.output,
                    self.arch,
                    old_offset,
                    self.ctx.stack_offset,
                    self.os,
                );
            }
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

        // alya-lang/alya#171: restore the tag pushed above (every push
        // below is paired, so it sits on top here) and record it for
        // later `is null`/`== null` checks on this local. Pinned memo
        // slots are reused without new allocs (see let twin above).
        if assign_carries_tag {
            match self.arch {
                Architecture::X64 => {
                    self.output.push_str("    pop %rdx\n");
                    self.ctx.stack_offset -= 8;
                }
                Architecture::ARM64 => {
                    self.output.push_str("    ldr x1, [sp], #16\n");
                    self.ctx.stack_offset -= 16;
                }
            }
            let tag_slot = match self.ctx.tag_slot_memo.get(&name).cloned() {
                Some(off) => off,
                None => match self
                    .ctx
                    .variables
                    .get(&format!("var_tag_slot:{}", name))
                    .cloned()
                {
                    Some(VarType::Number(off)) => off,
                    _ => {
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        self.ctx.stack_offset
                    }
                },
            };
            match self.arch {
                Architecture::X64 => {
                    self.output
                        .push_str(&format!("    movl %edx, -{}(%rbp)\n", tag_slot));
                }
                Architecture::ARM64 => {
                    crate::codegen::arch::arm64::loads::emit_arm64_store_x29_offset(
                        &mut self.output,
                        "w1",
                        tag_slot,
                        "x9",
                    );
                }
            }
            // The variables-level record drives `is null`/`== null`
            // reloads; merges intersect it (divergent fresh offsets
            // drop), loops restore memo-pinned ones (see control.rs).
            self.ctx
                .variables
                .insert(format!("var_tag_slot:{}", name), VarType::Number(tag_slot));
        }

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
                            self.seed_map_struct_markers(&name, entries);
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
                                if is_null_expr(v, &self.ctx.variables) {
                                    if let Expr::String(field) = k {
                                        self.ctx.variables.insert(
                                            format!("map_field_null:{}", field),
                                            VarType::Null(0),
                                        );
                                        self.ctx.variables.insert(
                                            format!("map_null:{}.{}", name, field),
                                            VarType::Null(0),
                                        );
                                    }
                                }
                                if is_proven_int_expr(v, &self.ctx.variables)
                                    || matches!(v, Expr::Number(_))
                                {
                                    if let Expr::String(field) = k {
                                        self.ctx.variables.insert(
                                            format!("map_field_int:{}", field),
                                            VarType::Number(0),
                                        );
                                        self.ctx.variables.insert(
                                            format!("map_int:{}.{}", name, field),
                                            VarType::Number(0),
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
                            // Key-level vetoes (alya-lang/alya#138): mirror
                            // the inference negatives.
                            for (k, v) in entries {
                                if !is_string_expr(v, &self.ctx.variables) {
                                    if let Expr::String(field) = k {
                                        self.ctx.variables.insert(
                                            format!("map_nonstr_key:{}", field),
                                            VarType::Number(0),
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
                                || is_proven_int_expr(value, &self.ctx.variables)
                                || is_map_entry_int(value, &self.ctx.variables)
                            {
                                self.ctx
                                    .variables
                                    .insert(format!("var_is_int:{}", name), VarType::Number(0));
                            }
                            // #151: conversions consult `var_rebound_nonint`
                            // instead (parallel marker, retains untouched):
                            // a non-proven RHS may hold anything, so later
                            // int()/float() must take the heuristic path.
                            // Proven RHS clears (rebinds self-heal).
                            if is_proven_int_expr(value, &self.ctx.variables)
                                || is_map_entry_int(value, &self.ctx.variables)
                            {
                                self.ctx
                                    .variables
                                    .remove(&format!("var_rebound_nonint:{}", name));
                            } else {
                                self.ctx.variables.insert(
                                    format!("var_rebound_nonint:{}", name),
                                    VarType::Number(0),
                                );
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
        if !is_weak && self.value_needs_heap_retain(value) {
            // Retain when maybe-heap; direct only when proven.
            if self.value_proven_heap(value) {
                arch::emit_rc_retain_direct(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                arch::emit_rc_retain(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
        }
        // B1: named stores outlive the wrapping ring buffer.
        if string_store_needs_dup(value, &self.ctx.variables) {
            arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        }
        self.ctx.stack_offset -= temp_offset;
        let field_idx = self
            .resolve_struct_field_target(object, field)
            .map(|(_, idx)| idx)
            // NOTE (alya-lang/alya#182): silent fallback, see
            // FieldAccess (DCE-pruned structs).
            .unwrap_or(0);
        arch::emit_struct_field_set(&mut self.output, self.arch, field_idx);
    }

    pub(super) fn generate_index_assign(&mut self, array: &Expr, index: &Expr, value: &Expr) {
        // alya-lang/alya#163: any element write may change what the
        // map holds — void all per-key value-struct claims for the
        // root variable (conservative: keys are not tracked across
        // writes, so a stale claim could misdispatch a method call).
        let mut root = array;
        loop {
            match root {
                crate::ast::Expr::Identifier(name) => {
                    self.clear_map_struct_markers(name);
                    break;
                }
                crate::ast::Expr::Index { array: inner, .. } => {
                    root = inner;
                }
                crate::ast::Expr::FieldAccess { object, .. } => {
                    root = object;
                }
                _ => break,
            }
        }
        if let Some(sname) = self.get_expr_struct_name(array) {
            // Cross-module lookup (alya-lang/alya#180).
            let matched = find_operator_overload(&self.ctx.functions, &sname, "[]=");
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
            // alya-lang/alya#171: dynamic writes record the value's exact
            // runtime tag so later reads distinguish 0 from null (the
            // `set` runtime itself clears unknown entries). The tag is
            // fresh only for tag-carrying values (or slot-spilled
            // locals); anything else keeps the legacy untagged write.
            // Base/key are re-evaluated for the second call, so both must
            // be side-effect free.
            let set_repeatable =
                typeof_operand_is_repeatable(array) && typeof_operand_is_repeatable(index);
            let set_reg_tag = matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                && set_repeatable
                && is_tag_carrying_read(value, &self.ctx.variables);
            let set_slot_tag: Option<i32> = if set_reg_tag || !set_repeatable {
                None
            } else if let Expr::Identifier(id) = value {
                match self.ctx.variables.get(&format!("var_tag_slot:{}", id)) {
                    Some(VarType::Number(off)) => Some(*off),
                    _ => None,
                }
            } else {
                None
            };
            // Callee-saved spill for the fresh tag across the `set` call
            // (arg pushes would misroute a stack spill; calls clobber the
            // tag register). Slot-sourced tags re-read from memory later.
            if set_reg_tag {
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    push %rbx\n");
                        self.ctx.stack_offset += 8;
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    str x19, [sp, #-16]!\n");
                        self.ctx.stack_offset += 16;
                    }
                }
            }
            let actual_args = [array, index, value];
            for arg in actual_args.iter() {
                self.generate_expression(arg);
                if std::ptr::eq(*arg, value) && self.store_value_needs_retain(value) {
                    if self.value_proven_heap(value) {
                        // Proven heap: skip the syscall probe.
                        arch::emit_rc_retain_direct(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else if is_tag_carrying_read(value, &self.ctx.variables) {
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
                // alya-lang/alya#171: stash the fresh value tag in the
                // callee-saved scratch *after* the clobbering calls above
                // and before the push below (pushes preserve registers).
                if std::ptr::eq(*arg, value) && set_reg_tag {
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    movl %edx, %ebx\n");
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    mov w19, w1\n");
                        }
                    }
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
            // alya-lang/alya#171: write the stashed tag with a second
            // call. Slot-sourced tags reload from memory; reg-sourced
            // ones restore from the callee-saved scratch. Static shapes
            // below are skipped when this fired (the runtime tag agrees
            // with — and outranks — static may-facts).
            let mut set_tag_fired = false;
            if set_reg_tag || set_slot_tag.is_some() {
                match self.arch {
                    Architecture::X64 => {
                        if set_reg_tag {
                            self.output.push_str("    movl %ebx, %edx\n");
                        } else if let Some(off) = set_slot_tag {
                            self.output
                                .push_str(&format!("    movl -{}(%rbp), %edx\n", off));
                        }
                    }
                    Architecture::ARM64 => {
                        if set_reg_tag {
                            self.output.push_str("    mov w1, w19\n");
                        } else if let Some(off) = set_slot_tag {
                            crate::codegen::arch::arm64::loads::emit_arm64_load_x29_offset(
                                &mut self.output,
                                "w1",
                                off,
                                "x9",
                            );
                        }
                    }
                }
                self.generate_expression(array);
                arch::emit_push_temp(&mut self.output, self.arch);
                self.generate_expression(index);
                arch::emit_push_temp(&mut self.output, self.arch);
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movl %edx, %eax\n");
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    str x1, [sp, #-16]!\n");
                        self.ctx.stack_offset += 16;
                    }
                }
                arch::emit_function_call(
                    &mut self.output,
                    self.arch,
                    "map_set_tag",
                    3,
                    self.ctx.stack_offset,
                    self.os,
                );
                if matches!(self.arch, Architecture::ARM64) {
                    self.ctx.stack_offset -= 16;
                }
                if set_reg_tag {
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    pop %rbx\n");
                            self.ctx.stack_offset -= 8;
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    ldr x19, [sp], #16\n");
                            self.ctx.stack_offset -= 16;
                        }
                    }
                }
                set_tag_fired = true;
            }
            // Record a value-kind tag for literal writes so variable-key
            // reads can dispatch on it. Only fully static shapes qualify
            // (identifier map, literal key, literal value): re-evaluating
            // anything else could duplicate side effects. Dynamics leave
            // the tag cleared by `set` itself (unknown = 0), unless the
            // dynamic write above already recorded the exact runtime tag.
            if !set_tag_fired {
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
            } else if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                // Non-string write voids the whole-map string claim
                // (alya-lang/alya#39); mirrors the inference veto.
                self.ctx
                    .variables
                    .insert(format!("map_nonstr:{}", map_name), VarType::Number(0));
                // Key-level veto (alya-lang/alya#138); mirrors inference.
                self.ctx
                    .variables
                    .insert(format!("map_nonstr_key:{}", field), VarType::Number(0));
            } else if let Expr::String(field) = index {
                // Non-string write under a string key on a non-identifier
                // base: the whole-map veto has no variable to attach to,
                // but the key-level veto still applies.
                self.ctx
                    .variables
                    .insert(format!("map_nonstr_key:{}", field), VarType::Number(0));
            }
            let field_str = match index {
                Expr::String(field) => Some(field.clone()),
                Expr::Identifier(k) => {
                    match self.ctx.variables.get(&format!("var_str_val:{}", k)) {
                        Some(VarType::StringLabel(s)) => Some(s.clone()),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let (Expr::Identifier(map_name), Some(field)) = (array, field_str) {
                if is_null_expr(value, &self.ctx.variables) {
                    self.ctx
                        .variables
                        .insert(format!("map_field_null:{}", field), VarType::Null(0));
                    self.ctx
                        .variables
                        .insert(format!("map_null:{}.{}", map_name, field), VarType::Null(0));
                    self.ctx
                        .variables
                        .remove(&format!("map_int:{}.{}", map_name, field));
                } else if is_proven_int_expr(value, &self.ctx.variables)
                    || matches!(value, Expr::Number(_))
                {
                    self.ctx
                        .variables
                        .insert(format!("map_field_int:{}", field), VarType::Number(0));
                    self.ctx.variables.insert(
                        format!("map_int:{}.{}", map_name, field),
                        VarType::Number(0),
                    );
                    self.ctx
                        .variables
                        .remove(&format!("map_null:{}.{}", map_name, field));
                } else {
                    self.ctx
                        .variables
                        .remove(&format!("map_null:{}.{}", map_name, field));
                    self.ctx
                        .variables
                        .remove(&format!("map_int:{}.{}", map_name, field));
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
        } else if !is_array_fold_true(array, &self.ctx.variables) {
            // Dynamically-unknown container with a non-string index
            // (alya-lang/alya#84): it may be a map (e.g. an untyped param
            // holding one) or an array. Usage-based array markings
            // (`s[k] = v` marks `s` array) are may-facts, so only exact
            // evidence (`is_array_fold_true`) earns the static fast path;
            // everything else dispatches on the container header at
            // runtime (map tag -> `set`, anything else -> the exact
            // array-store sequence, preserving its OOB fault).
            // The value retain mirrors the map path (tag-guarded for
            // unknown kinds: safe for heap, skips faulting int retains).
            let kind_expr = Expr::Number(value_kind_tag(value, &self.ctx.variables) as i128);
            let actual_args = [array, index, value, &kind_expr];
            for arg in actual_args.iter() {
                self.generate_expression(arg);
                if std::ptr::eq(*arg, value) && self.store_value_needs_retain(value) {
                    if self.value_proven_heap(value) {
                        // Proven heap: skip the syscall probe.
                        arch::emit_rc_retain_direct(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else if is_tag_carrying_read(value, &self.ctx.variables) {
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
                "dyn_index_assign",
                4,
                self.ctx.stack_offset,
                self.os,
            );
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
                if self.value_proven_heap(value) {
                    // Proven heap: skip the syscall probe.
                    arch::emit_rc_retain_direct(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
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
            if string_store_needs_dup(value, &self.ctx.variables) {
                arch::emit_str_store(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
            }
            self.ctx.stack_offset -= temp_offset * 2;
            let set_kind = value_kind_tag(value, &self.ctx.variables);
            arch::emit_array_set(&mut self.output, self.arch, set_kind);
        }
    }
}
