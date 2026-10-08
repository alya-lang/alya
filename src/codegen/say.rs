use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    call_returns_known_int, escape_string, is_array_expr, is_array_kind_read,
    is_dynamic_element_read, is_float_expr, is_map_expr, is_map_read_index, is_null_expr,
    is_string_array, is_string_expr, is_tag_carrying_read, is_unsigned_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::kinds::{KIND_ARRAY, KIND_FLOAT, KIND_INT, KIND_MAP, KIND_STRING};
use crate::codegen::target::Architecture;

/// Display a string array as `[a, b]` via existing `join` + concat.
/// Avoids printing the raw array pointer with `%lld` (e.g. `2355014589440`).
fn string_array_display_expr(arr: Expr) -> Expr {
    let join_call = Expr::Call {
        name: "join".into(),
        args: vec![arr, Expr::String(", ".into())],
    };
    let left = Expr::Binary {
        left: Box::new(Expr::String("[".into())),
        op: BinaryOp::Add,
        right: Box::new(join_call),
    };
    Expr::Binary {
        left: Box::new(left),
        op: BinaryOp::Add,
        right: Box::new(Expr::String("]".into())),
    }
}

impl CodeGen {
    /// Emit a runtime-guarded map print for a value already in the
    /// accumulator (`%rax` / `%eax` / `x0`).
    ///
    /// Statically-typed maps print directly elsewhere, but values folded
    /// to map statically (e.g. `json_parse`, which returns any JSON type,
    /// or a `let`-bound variable holding its result) must be verified at
    /// runtime: feeding a scalar into `alya_print_map` faults. Guard:
    /// null prints "null"; header magic `0x5A110002` at the object header
    /// prints as map; anything else falls back to runtime classification
    /// (string -> `%s`, otherwise -> `%lld`). Large-int and float dynamics
    /// can still fault on the header read: without value tags they are
    /// indistinguishable from heap pointers (see issue #39).
    pub(crate) fn emit_guarded_map_print(&mut self) {
        let l_dyn = self.ctx.next_label();
        let l_str = self.ctx.next_label();
        let l_null = self.ctx.next_label();
        let l_end = self.ctx.next_label();
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    test %rax, %rax\n");
                self.output.push_str(&format!("    jz {}\n", l_null));
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_dyn));
                self.output.push_str("    mov $0x00007fffffffffff, %rdx\n");
                self.output.push_str("    cmp %rdx, %rax\n");
                self.output.push_str(&format!("    ja {}\n", l_dyn));
                self.output.push_str("    movl -16(%rax), %edx\n");
                self.output.push_str("    cmpl $0x5A110002, %edx\n");
                self.output.push_str(&format!("    jne {}\n", l_dyn));
            }
            Architecture::ARM64 => {
                self.output.push_str(&format!("    cbz x0, {}\n", l_null));
                self.output.push_str("    movz x1, #1, lsl #16\n");
                self.output.push_str("    cmp x0, x1\n");
                self.output.push_str(&format!("    b.ls {}\n", l_dyn));
                self.output.push_str("    ldr x1, [x0, #-16]\n");
                // Mask to the low 32-bit tag (high word is GC state;
                // x64 uses movl here).
                self.output.push_str("    uxtw x1, w1\n");
                self.output.push_str("    movz x2, #0x0002\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.ne {}\n", l_dyn));
            }
        }
        arch::emit_print_map(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
        self.output.push('\n');
        arch::emit_jump(&mut self.output, self.arch, &l_end);
        // Null prints as "null" (mirrors the VarType::Null arm).
        self.output.push_str(&format!("{}:\n", l_null));
        let fmt_null_label = self.ctx.next_string_label();
        self.emit_rodata_section();
        self.output.push_str(&format!("{}:\n", fmt_null_label));
        self.emit_string_directive("null\\n");
        self.output.push_str(".text\n");
        arch::emit_say_str_lit(
            &mut self.output,
            self.arch,
            &fmt_null_label,
            self.ctx.stack_offset,
            self.os,
        );
        self.output.push('\n');
        arch::emit_jump(&mut self.output, self.arch, &l_end);
        // Dynamic fallback: classify at runtime (string -> %s,
        // anything else -> %lld). One push, one pop per path.
        self.output.push_str(&format!("{}:\n", l_dyn));
        arch::emit_push_temp(&mut self.output, self.arch);
        self.emit_runtime_classify(self.os);
        arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
        arch::emit_cond_jump(
            &mut self.output,
            self.arch,
            BinaryOp::Equal,
            false,
            &l_str,
            false,
        );
        arch::emit_pop_temp(&mut self.output, self.arch);
        let fmt_dyn_int_label = self.ctx.next_string_label();
        self.emit_rodata_section();
        self.output.push_str(&format!("{}:\n", fmt_dyn_int_label));
        self.emit_string_directive("%lld\\n");
        self.output.push_str(".text\n");
        arch::emit_say_acc(
            &mut self.output,
            self.arch,
            &fmt_dyn_int_label,
            self.ctx.stack_offset,
            self.os,
        );
        arch::emit_jump(&mut self.output, self.arch, &l_end);
        self.output.push_str(&format!("{}:\n", l_str));
        arch::emit_pop_temp(&mut self.output, self.arch);
        let fmt_dyn_str_label = self.ctx.next_string_label();
        self.emit_rodata_section();
        self.output.push_str(&format!("{}:\n", fmt_dyn_str_label));
        self.emit_string_directive("%s\\n");
        self.output.push_str(".text\n");
        arch::emit_say_acc(
            &mut self.output,
            self.arch,
            &fmt_dyn_str_label,
            self.ctx.stack_offset,
            self.os,
        );
        self.output.push_str(&format!("{}:\n", l_end));
    }

    pub(crate) fn generate_say(&mut self, expr: &Expr) {
        match expr {
            Expr::Null => {
                let label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", label));
                self.emit_string_directive("null\\n");
                self.output.push_str(".text\n");

                arch::emit_say_str_lit(
                    &mut self.output,
                    self.arch,
                    &label,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::String(s) => {
                let label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", label));
                self.emit_string_directive(&format!("{}\\n", escape_string(s)));
                self.output.push_str(".text\n");

                arch::emit_say_str_lit(
                    &mut self.output,
                    self.arch,
                    &label,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::InterpolatedString(parts) => {
                let mut format_str = String::new();
                let mut exprs: Vec<Expr> = Vec::new();
                let mut is_floats = Vec::new();

                for part in parts {
                    match part {
                        Expr::String(s) => {
                            format_str.push_str(&escape_string(s).replace('%', "%%"));
                        }
                        Expr::Call { name, args } if name.starts_with("__alya_format:") => {
                            let spec = &name["__alya_format:".len()..];
                            let arg = &args[0];
                            if spec.ends_with('f') && spec.starts_with('.') {
                                format_str.push_str(&format!("%{}", spec));
                                exprs.push(arg.clone());
                                is_floats.push(true);
                            } else if spec.starts_with('0')
                                && spec.chars().skip(1).all(|c| c.is_ascii_digit())
                            {
                                // B4: unsigned zero-padded ints print with %llu.
                                let conv = if is_unsigned_expr(arg, &self.ctx.variables) {
                                    "llu"
                                } else {
                                    "lld"
                                };
                                format_str.push_str(&format!("%{}{}", spec, conv));
                                exprs.push(arg.clone());
                                is_floats.push(false);
                            } else if spec == "#x" || spec == "x" {
                                format_str.push_str("%#llx");
                                exprs.push(arg.clone());
                                is_floats.push(false);
                            } else if spec == "#b" || spec == "b" {
                                format_str.push_str("%s");
                                exprs.push(Expr::Call {
                                    name: "format_binary".into(),
                                    args: vec![arg.clone()],
                                });
                                is_floats.push(false);
                            } else if let Some(width) = spec.strip_prefix('>') {
                                format_str.push_str(&format!("%{}s", width));
                                exprs.push(arg.clone());
                                is_floats.push(false);
                            } else if let Some(width) = spec.strip_prefix('<') {
                                format_str.push_str(&format!("%-{}s", width));
                                exprs.push(arg.clone());
                                is_floats.push(false);
                            } else {
                                // B4: unsigned ints print with %llu.
                                if is_unsigned_expr(arg, &self.ctx.variables) {
                                    format_str.push_str("%llu");
                                } else {
                                    format_str.push_str("%lld");
                                }
                                exprs.push(arg.clone());
                                is_floats.push(false);
                            }
                        }
                        _ => {
                            if is_null_expr(part, &self.ctx.variables) {
                                format_str.push_str("null");
                            } else {
                                let struct_to_string = if let Some(sname) =
                                    self.get_expr_struct_name(part)
                                {
                                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                                    let bare_sname =
                                        bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                                    let cand1 = format!("{}__{}", sname, "to_string");
                                    let cand2 = format!("{}__{}", bare_sname, "to_string");
                                    if self.ctx.functions.contains(&cand1) {
                                        Some(cand1)
                                    } else if self.ctx.functions.contains(&cand2) {
                                        Some(cand2)
                                    } else {
                                        self.ctx
                                            .functions
                                            .iter()
                                            .find(|f| f.ends_with("__to_string"))
                                            .cloned()
                                    }
                                } else {
                                    None
                                };
                                if let Some(ts_func) = struct_to_string {
                                    format_str.push_str("%s");
                                    exprs.push(Expr::Call {
                                        name: ts_func,
                                        args: vec![part.clone()],
                                    });
                                    is_floats.push(false);
                                } else if is_string_array(part, &self.ctx.variables) {
                                    format_str.push_str("%s");
                                    exprs.push(string_array_display_expr(part.clone()));
                                    is_floats.push(false);
                                } else if matches!(part, Expr::Index { .. })
                                    && is_tag_carrying_read(part, &self.ctx.variables)
                                    && !is_string_expr(part, &self.ctx.variables)
                                    && !is_float_expr(part, &self.ctx.variables)
                                {
                                    // Tag-carrying reads with unknown static
                                    // type route through str() (tag dispatch,
                                    // alya-lang/alya#39 Phase 2b) instead of
                                    // printing raw bits with %lld. Proven
                                    // string/float parts keep their arms.
                                    format_str.push_str("%s");
                                    exprs.push(Expr::Call {
                                        name: "str".into(),
                                        args: vec![part.clone()],
                                    });
                                    is_floats.push(false);
                                } else {
                                    let is_flt = is_float_expr(part, &self.ctx.variables);
                                    let is_str = is_string_expr(part, &self.ctx.variables);
                                    if is_str {
                                        format_str.push_str("%s");
                                        exprs.push(part.clone());
                                        is_floats.push(false);
                                    } else if is_flt {
                                        // B4: floats print shortest-round-trip
                                        // via str() (fn_str_from_float), so
                                        // the format takes a string here.
                                        format_str.push_str("%s");
                                        exprs.push(Expr::Call {
                                            name: "str".into(),
                                            args: vec![part.clone()],
                                        });
                                        is_floats.push(false);
                                    } else if is_unsigned_expr(part, &self.ctx.variables) {
                                        // B4: u64 interpolation prints unsigned.
                                        format_str.push_str("%llu");
                                        exprs.push(part.clone());
                                        is_floats.push(false);
                                    } else {
                                        // #93: statically-unknown holes render
                                        // through str() (value, not pointer),
                                        // mirroring the tag-carrying arm
                                        // above. Ints/bools/null/floats/maps
                                        // render byte-identically to %lld;
                                        // only dynamic strings change.
                                        format_str.push_str("%s");
                                        exprs.push(Expr::Call {
                                            name: "str".into(),
                                            args: vec![part.clone()],
                                        });
                                        is_floats.push(false);
                                    }
                                }
                            }
                        }
                    }
                }
                format_str.push_str("\\n");

                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                self.emit_string_directive(&format_str);
                self.output.push_str(".text\n");

                let initial_stack_offset = self.ctx.stack_offset;
                let word_size: i32 = match self.arch {
                    Architecture::ARM64 => 16,
                    _ => 8,
                };
                for (idx, expr) in exprs.iter().enumerate() {
                    self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                    self.generate_expression(expr);
                    arch::emit_push_temp(&mut self.output, self.arch);
                }
                self.ctx.stack_offset = initial_stack_offset;

                arch::emit_say_interpolated(
                    &mut self.output,
                    self.arch,
                    &fmt_label,
                    &is_floats,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::Binary { left, op, right } => {
                if matches!(op, BinaryOp::Add)
                    && (is_string_expr(left, &self.ctx.variables)
                        || is_string_expr(right, &self.ctx.variables))
                {
                    self.generate_string_concat(left, right);
                    let fmt_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_label));
                    self.emit_string_directive("%s\\n");
                    self.output.push_str(".text\n");

                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    self.output.push('\n');
                    return;
                }
                // #94: arithmetic over maybe-strings dispatches to concat
                // at runtime (see generate_dynamic_add), but the optimistic
                // is_number rule still types the result int here. Route
                // through str() so the value prints, not the pointer.
                // Ints render byte-identically either way.
                if matches!(op, BinaryOp::Add)
                    && Self::add_may_hold_string(left, &self.ctx.variables)
                    && Self::add_may_hold_string(right, &self.ctx.variables)
                {
                    self.generate_say(&Expr::Call {
                        name: "str".into(),
                        args: vec![Expr::Binary {
                            left: left.clone(),
                            op: *op,
                            right: right.clone(),
                        }],
                    });
                    return;
                }
                let is_flt = is_float_expr(expr, &self.ctx.variables);
                self.generate_expression(expr);

                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                if is_flt {
                    self.emit_string_directive("%s\\n");
                } else if is_unsigned_expr(expr, &self.ctx.variables) {
                    // B4: unsigned arithmetic results print unsigned.
                    self.emit_string_directive("%llu\\n");
                } else {
                    self.emit_string_directive("%lld\\n");
                }
                self.output.push_str(".text\n");

                if is_flt {
                    arch::emit_say_float(
                        &mut self.output,
                        self.arch,
                        &fmt_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else {
                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
                self.output.push('\n');
            }
            Expr::Identifier(name) => {
                // Shared module globals live in BSS; the variables map
                // only carries kind markers with stale stack offsets for
                // them, so resolve globals before the offset-based paths
                // below (alya-lang/alya#48).
                if self.ctx.globals.contains_key(name) {
                    let is_int = self
                        .ctx
                        .variables
                        .contains_key(&format!("var_is_int:{}", name));
                    let kind = self.ctx.variables.get(name).cloned();
                    let fmt_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_label));
                    match kind {
                        Some(VarType::Float(_)) => {
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");
                            if let Some((symbol, _)) = self.ctx.globals.get(name).cloned() {
                                arch::emit_load_global(
                                    &mut self.output,
                                    self.arch,
                                    &symbol,
                                    self.os,
                                );
                            }
                            arch::emit_say_float(
                                &mut self.output,
                                self.arch,
                                &fmt_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                            return;
                        }
                        Some(VarType::StringOffset(_)) | Some(VarType::StringLabel(_)) => {
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");
                            if let Some((symbol, _)) = self.ctx.globals.get(name).cloned() {
                                arch::emit_load_global(
                                    &mut self.output,
                                    self.arch,
                                    &symbol,
                                    self.os,
                                );
                            }
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                            return;
                        }
                        _ if is_int => {
                            // B4: u64 globals print unsigned.
                            let fmt = if self
                                .ctx
                                .variables
                                .contains_key(&format!("var_is_uint:{}", name))
                            {
                                "%llu\\n"
                            } else {
                                "%lld\\n"
                            };
                            self.emit_string_directive(fmt);
                            self.output.push_str(".text\n");
                            if let Some((symbol, _)) = self.ctx.globals.get(name).cloned() {
                                arch::emit_load_global(
                                    &mut self.output,
                                    self.arch,
                                    &symbol,
                                    self.os,
                                );
                            }
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                            return;
                        }
                        _ => {
                            // Unknown global kind: top-level lets generate
                            // after function bodies (#55-C), so kind markers
                            // are absent here. Load the value and classify
                            // at runtime (string -> %s, else -> %lld),
                            // mirroring the dynamic fallback above.
                            // NOTE: the outer fmt_label above stays an empty
                            // rodata label (harmless); switch back to .text
                            // before emitting any code.
                            self.output.push_str(".text\n");
                            if let Some((symbol, _)) = self.ctx.globals.get(name).cloned() {
                                arch::emit_load_global(
                                    &mut self.output,
                                    self.arch,
                                    &symbol,
                                    self.os,
                                );
                                let l_gstr = self.ctx.next_label();
                                let l_gend = self.ctx.next_label();
                                arch::emit_push_temp(&mut self.output, self.arch);
                                self.emit_runtime_classify(self.os);
                                arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
                                arch::emit_cond_jump(
                                    &mut self.output,
                                    self.arch,
                                    BinaryOp::Equal,
                                    false,
                                    &l_gstr,
                                    false,
                                );
                                arch::emit_pop_temp(&mut self.output, self.arch);
                                let fmt_gint_label = self.ctx.next_string_label();
                                self.emit_rodata_section();
                                self.output.push_str(&format!("{}:\n", fmt_gint_label));
                                self.emit_string_directive("%lld\\n");
                                self.output.push_str(".text\n");
                                arch::emit_say_acc(
                                    &mut self.output,
                                    self.arch,
                                    &fmt_gint_label,
                                    self.ctx.stack_offset,
                                    self.os,
                                );
                                arch::emit_jump(&mut self.output, self.arch, &l_gend);
                                self.output.push_str(&format!("{}:\n", l_gstr));
                                arch::emit_pop_temp(&mut self.output, self.arch);
                                let fmt_gstr_label = self.ctx.next_string_label();
                                self.emit_rodata_section();
                                self.output.push_str(&format!("{}:\n", fmt_gstr_label));
                                self.emit_string_directive("%s\\n");
                                self.output.push_str(".text\n");
                                arch::emit_say_acc(
                                    &mut self.output,
                                    self.arch,
                                    &fmt_gstr_label,
                                    self.ctx.stack_offset,
                                    self.os,
                                );
                                self.output.push_str(&format!("{}:\n", l_gend));
                                self.output.push('\n');
                                return;
                            }
                        }
                    }
                }
                if let Some(var_type) = self.ctx.variables.get(name).cloned() {
                    match var_type {
                        VarType::StringLabel(label) => {
                            let fmt_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_label));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");

                            arch::emit_say_str(
                                &mut self.output,
                                self.arch,
                                &label,
                                &fmt_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::StringOffset(offset) => {
                            let fmt_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_label));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");

                            arch::emit_say_offset(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                                &fmt_label,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::Number(offset) => {
                            if self
                                .ctx
                                .variables
                                .contains_key(&format!("var_is_int:{}", name))
                            {
                                let fmt_int_label = self.ctx.next_string_label();
                                self.emit_rodata_section();
                                self.output.push_str(&format!("{}:\n", fmt_int_label));
                                // B4: u64-annotated locals print unsigned.
                                let fmt = if self
                                    .ctx
                                    .variables
                                    .contains_key(&format!("var_is_uint:{}", name))
                                {
                                    "%llu\\n"
                                } else {
                                    "%lld\\n"
                                };
                                self.emit_string_directive(fmt);
                                self.output.push_str(".text\n");
                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    self.ctx.stack_offset,
                                );
                                arch::emit_say_acc(
                                    &mut self.output,
                                    self.arch,
                                    &fmt_int_label,
                                    self.ctx.stack_offset,
                                    self.os,
                                );
                                self.output.push('\n');
                                return;
                            }
                            // Statically-unknown dynamics are recorded as
                            // Number. Call-bound ones (`let v = F(...)`
                            // with a dynamically-typed call result, e.g.
                            // json_parse which returns any JSON type) need
                            // the same guarded print as Map-typed dynamics
                            // (null/map/string/small-int dispatch); a
                            // string-vs-int check alone prints null as 0
                            // and maps as pointers. Other Number dynamics
                            // (e.g. int loop vars, where 0 must print as
                            // 0 not null) keep the string/int path.
                            if self
                                .ctx
                                .variables
                                .contains_key(&format!("call_bound:{}", name))
                            {
                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    self.ctx.stack_offset,
                                );
                                self.emit_guarded_map_print();
                                self.output.push('\n');
                                return;
                            }
                            // Map reads with variable keys would print
                            // string pointers as integers. Classify at
                            // runtime: real integers take the identical %lld
                            // path, so behavior is unchanged for them, except
                            // for integers that numerically fall inside the
                            // rodata/str-buf windows (documented edge).
                            let l_dyn_str = self.ctx.next_label();
                            let l_dyn_end = self.ctx.next_label();
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            self.emit_runtime_classify(self.os);
                            arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
                            arch::emit_cond_jump(
                                &mut self.output,
                                self.arch,
                                BinaryOp::Equal,
                                false,
                                &l_dyn_str,
                                false,
                            );
                            let fmt_int_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_int_label));
                            self.emit_string_directive("%lld\\n");
                            self.output.push_str(".text\n");
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_int_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            arch::emit_jump(&mut self.output, self.arch, &l_dyn_end);
                            self.output.push_str(&format!("{}:\n", l_dyn_str));
                            let fmt_dyn_str_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_dyn_str_label));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_dyn_str_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push_str(&format!("{}:\n", l_dyn_end));
                            self.output.push('\n');
                        }
                        VarType::Float(offset) => {
                            let fmt_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_label));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");

                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_say_float(
                                &mut self.output,
                                self.arch,
                                &fmt_label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::Array(offset) => {
                            if is_string_array(&Expr::Identifier(name.clone()), &self.ctx.variables)
                            {
                                self.generate_say(&string_array_display_expr(Expr::Identifier(
                                    name.clone(),
                                )));
                                return;
                            }
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_print_array(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::Map(offset) => {
                            // Map-typed variables can hold non-map dynamics
                            // at runtime (`let v = json_parse(...)` folds to
                            // map statically but returns any JSON type), so
                            // verify before printing (see
                            // emit_guarded_map_print). Genuine maps take the
                            // identical print path as before.
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            self.emit_guarded_map_print();
                            self.output.push('\n');
                        }
                        VarType::Struct { offset, .. } => {
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_print_struct(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::Interface { offset, .. } => {
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            // Unwrap the fat pointer to the concrete data
                            // pointer for printing (cells are pointer-sized
                            // per arch).
                            match self.arch {
                                Architecture::X64 => {
                                    self.output.push_str("    movq (%rax), %rax\n");
                                }
                                Architecture::ARM64 => {
                                    self.output.push_str("    ldr x0, [x0]\n");
                                }
                            }
                            arch::emit_print_struct(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                        VarType::Null(_) => {
                            let label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", label));
                            self.emit_string_directive("null\\n");
                            self.output.push_str(".text\n");

                            arch::emit_say_str_lit(
                                &mut self.output,
                                self.arch,
                                &label,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push('\n');
                        }
                    }
                }
            }
            Expr::StructInit { .. } => {
                self.generate_expression(expr);
                arch::emit_print_struct(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::Call { name, .. } if self.ctx.structs.contains_key(name) => {
                self.generate_expression(expr);
                arch::emit_print_struct(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::Array(_) => {
                if is_string_array(expr, &self.ctx.variables) {
                    self.generate_say(&string_array_display_expr(expr.clone()));
                    return;
                }
                self.generate_expression(expr);
                arch::emit_print_array(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
                self.output.push('\n');
            }
            Expr::Map(_) => {
                self.generate_expression(expr);
                arch::emit_print_map(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
                self.output.push('\n');
            }
            Expr::Float(n) => {
                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                self.emit_string_directive("%s\\n");
                self.output.push_str(".text\n");

                arch::emit_load_float(&mut self.output, self.arch, *n);
                arch::emit_say_float(
                    &mut self.output,
                    self.arch,
                    &fmt_label,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            Expr::Number(n) => {
                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                // B4: u64 max (18446744073709551615) must print unsigned.
                let fmt = if *n > i64::MAX as i128 {
                    "%llu\\n"
                } else {
                    "%lld\\n"
                };
                self.emit_string_directive(fmt);
                self.output.push_str(".text\n");

                arch::emit_say_num_const(
                    &mut self.output,
                    self.arch,
                    *n as i64,
                    &fmt_label,
                    self.ctx.stack_offset,
                    self.os,
                );
                self.output.push('\n');
            }
            _ => {
                if let Some(sname) = self.get_expr_struct_name(expr) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let cand1 = format!("{}__{}", sname, "to_string");
                    let cand2 = format!("{}__{}", bare_sname, "to_string");
                    let matched = if self.ctx.functions.contains(&cand1) {
                        Some(cand1)
                    } else if self.ctx.functions.contains(&cand2) {
                        Some(cand2)
                    } else {
                        // Prefer the value's own struct overload (aliased
                        // imports duplicate helpers under a namespace
                        // prefix that may use `::`; normalize before
                        // comparing); fall back to any `__to_string` only
                        // when the struct defines none of its own.
                        let want1 = format!("__{}__to_string", sname);
                        let want2 = format!("__{}__to_string", bare_sname);
                        self.ctx
                            .functions
                            .iter()
                            .find(|f| {
                                let n = f.replace("::", "__");
                                n.ends_with(&want1) || n.ends_with(&want2)
                            })
                            .or_else(|| {
                                self.ctx
                                    .functions
                                    .iter()
                                    .find(|f| f.ends_with("__to_string"))
                            })
                            .cloned()
                    };
                    if let Some(call_name) = matched {
                        self.generate_say(&Expr::Call {
                            name: call_name,
                            args: vec![expr.clone()],
                        });
                        return;
                    }
                }

                if is_null_expr(expr, &self.ctx.variables) {
                    let label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", label));
                    self.emit_string_directive("null\\n");
                    self.output.push_str(".text\n");

                    arch::emit_say_str_lit(
                        &mut self.output,
                        self.arch,
                        &label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    self.output.push('\n');
                    return;
                }

                if is_string_array(expr, &self.ctx.variables) {
                    self.generate_say(&string_array_display_expr(expr.clone()));
                    return;
                }

                if is_map_expr(expr, &self.ctx.variables) {
                    // Dynamically-typed values folded to map (e.g. bare
                    // `json_parse`, which returns any JSON type) must be
                    // verified at runtime: feeding a scalar into
                    // alya_print_map faults. See emit_guarded_map_print.
                    self.generate_expression(expr);
                    self.emit_guarded_map_print();
                    self.output.push('\n');
                    return;
                }

                if is_array_expr(expr, &self.ctx.variables) {
                    self.generate_expression(expr);
                    arch::emit_print_array(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    self.output.push('\n');
                    return;
                }

                if let Expr::Index { .. } = expr {
                    // Index reads whose value type is statically unknown
                    // (e.g. variable keys into maps) would print heap
                    // pointers as integers. Classify the value at runtime;
                    // proven string/array/map/float results keep their
                    // existing paths above.
                    if !is_string_expr(expr, &self.ctx.variables)
                        && !is_array_expr(expr, &self.ctx.variables)
                        && !is_map_expr(expr, &self.ctx.variables)
                        && !is_float_expr(expr, &self.ctx.variables)
                    {
                        let l_idx_str = self.ctx.next_label();
                        let l_idx_end = self.ctx.next_label();
                        // NOTE: no stack_offset adjustments here. The push
                        // below is balanced by exactly one pop on every
                        // runtime path, and nothing emitted between them
                        // reads stack_offset, so the tracked value stays
                        // correct for the print sequences (which run after
                        // their pop, i.e. at net-zero depth).
                        self.generate_expression(expr);
                        arch::emit_push_temp(&mut self.output, self.arch);
                        // fn_get returns the entry kind tag alongside the value
                        // (x64: %edx, arm64: w1). Array loads deliver the
                        // slot kind the same way on x64 (Phase 1, #39).
                        // Tagged values dispatch directly; unknown falls
                        // through to the legacy pointer-range classifier
                        // below.
                        // Tags: 0 unknown, 1 int, 2 float, 3 string,
                        // 4 array, 5 map.
                        // NOTE: only map-routed reads (fn_get) and plain
                        // array-identifier reads carry a tag. Other shapes
                        // leave the tag register holding the index, so they
                        // must skip tag dispatch.
                        let carries_kind = is_map_read_index(expr, &self.ctx.variables)
                            || is_array_kind_read(expr, &self.ctx.variables)
                            || is_dynamic_element_read(expr, &self.ctx.variables);
                        let (l_tag_flt, l_tag_str2, l_tag_arr, l_tag_map, l_tag_int) =
                            if carries_kind {
                                let flt = self.ctx.next_label();
                                let s2 = self.ctx.next_label();
                                let arr = self.ctx.next_label();
                                let mp = self.ctx.next_label();
                                let it = self.ctx.next_label();
                                if matches!(self.arch, Architecture::X64) {
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    je {}\n", flt));
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_STRING));
                                    self.output.push_str(&format!("    je {}\n", s2));
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_ARRAY));
                                    self.output.push_str(&format!("    je {}\n", arr));
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_MAP));
                                    self.output.push_str(&format!("    je {}\n", mp));
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_INT));
                                    self.output.push_str(&format!("    je {}\n", it));
                                } else {
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    b.eq {}\n", flt));
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_STRING));
                                    self.output.push_str(&format!("    b.eq {}\n", s2));
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_ARRAY));
                                    self.output.push_str(&format!("    b.eq {}\n", arr));
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_MAP));
                                    self.output.push_str(&format!("    b.eq {}\n", mp));
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_INT));
                                    self.output.push_str(&format!("    b.eq {}\n", it));
                                }
                                (Some(flt), Some(s2), Some(arr), Some(mp), Some(it))
                            } else {
                                (None, None, None, None, None)
                            };
                        self.emit_runtime_classify(self.os);
                        arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            BinaryOp::Equal,
                            false,
                            &l_idx_str,
                            false,
                        );
                        arch::emit_pop_temp(&mut self.output, self.arch);
                        let fmt_idx_int_label = self.ctx.next_string_label();
                        self.emit_rodata_section();
                        self.output.push_str(&format!("{}:\n", fmt_idx_int_label));
                        self.emit_string_directive("%lld\\n");
                        self.output.push_str(".text\n");
                        arch::emit_say_acc(
                            &mut self.output,
                            self.arch,
                            &fmt_idx_int_label,
                            self.ctx.stack_offset,
                            self.os,
                        );
                        arch::emit_jump(&mut self.output, self.arch, &l_idx_end);
                        self.output.push_str(&format!("{}:\n", l_idx_str));
                        arch::emit_pop_temp(&mut self.output, self.arch);
                        let fmt_idx_str_label = self.ctx.next_string_label();
                        self.emit_rodata_section();
                        self.output.push_str(&format!("{}:\n", fmt_idx_str_label));
                        self.emit_string_directive("%s\\n");
                        self.output.push_str(".text\n");
                        arch::emit_say_acc(
                            &mut self.output,
                            self.arch,
                            &fmt_idx_str_label,
                            self.ctx.stack_offset,
                            self.os,
                        );
                        self.output.push_str(&format!("{}:\n", l_idx_end));
                        // Tagged fast paths. Each pops the saved
                        // value and prints with the tag-correct runtime.
                        if let (Some(t_flt), Some(t_str2), Some(t_arr), Some(t_map), Some(t_int)) =
                            (l_tag_flt, l_tag_str2, l_tag_arr, l_tag_map, l_tag_int)
                        {
                            let l_final = self.ctx.next_label();
                            arch::emit_jump(&mut self.output, self.arch, &l_final);
                            // float
                            self.output.push_str(&format!("{}:\n", t_flt));
                            arch::emit_pop_temp(&mut self.output, self.arch);
                            let fmt_tag_flt = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_tag_flt));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");
                            arch::emit_say_float(
                                &mut self.output,
                                self.arch,
                                &fmt_tag_flt,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            arch::emit_jump(&mut self.output, self.arch, &l_final);
                            // string
                            self.output.push_str(&format!("{}:\n", t_str2));
                            arch::emit_pop_temp(&mut self.output, self.arch);
                            let fmt_tag_str = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_tag_str));
                            self.emit_string_directive("%s\\n");
                            self.output.push_str(".text\n");
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_tag_str,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            arch::emit_jump(&mut self.output, self.arch, &l_final);
                            // array
                            self.output.push_str(&format!("{}:\n", t_arr));
                            arch::emit_pop_temp(&mut self.output, self.arch);
                            arch::emit_print_array(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            arch::emit_jump(&mut self.output, self.arch, &l_final);
                            // map
                            self.output.push_str(&format!("{}:\n", t_map));
                            arch::emit_pop_temp(&mut self.output, self.arch);
                            arch::emit_print_map(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            arch::emit_jump(&mut self.output, self.arch, &l_final);
                            // int
                            self.output.push_str(&format!("{}:\n", t_int));
                            arch::emit_pop_temp(&mut self.output, self.arch);
                            let fmt_tag_int = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_tag_int));
                            self.emit_string_directive("%lld\\n");
                            self.output.push_str(".text\n");
                            arch::emit_say_acc(
                                &mut self.output,
                                self.arch,
                                &fmt_tag_int,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            self.output.push_str(&format!("{}:\n", l_final));
                        }
                        self.output.push('\n');
                        return;
                    }
                }

                let is_str = is_string_expr(expr, &self.ctx.variables);
                let is_flt = is_float_expr(expr, &self.ctx.variables);
                // Calls with statically-unknown returns (e.g. a user
                // function returning an array element) may produce strings;
                // printing the raw pointer with %lld corrupts output (issue
                // #44). Classify at runtime like unknown dynamics. Calls
                // proven to return ints keep the direct %lld path.
                // Struct fields demoted to dynamic dispatch (mixed kinds,
                // alya-lang/alya#131/#132) are the same shape: no static
                // marker serves every instance, so classify the loaded
                // word at runtime instead of guessing %lld.
                let unknown_dynamic = match expr {
                    Expr::Call { name, .. } => {
                        !is_str
                            && !is_flt
                            && !is_map_expr(expr, &self.ctx.variables)
                            && !is_array_expr(expr, &self.ctx.variables)
                            && !is_null_expr(expr, &self.ctx.variables)
                            && !call_returns_known_int(name, &self.ctx.variables)
                    }
                    Expr::FieldAccess { .. } | Expr::OptionalFieldAccess { .. } => {
                        !is_str
                            && !is_flt
                            && !is_map_expr(expr, &self.ctx.variables)
                            && !is_array_expr(expr, &self.ctx.variables)
                            && !is_null_expr(expr, &self.ctx.variables)
                    }
                    _ => false,
                };
                self.generate_expression(expr);

                // Tag-carrying ternary results (Phase 2b, #39): the taken
                // arm's tag is fresh here (codegen materializes every
                // other arm). Float/string tags print exactly; anything
                // else falls through to the static handling below.
                // Value contract per arch: int/string in the int register,
                // float bits in the int register (x64/arm64 rebuild f64
                // from them). Qualified calls share the
                // contract: the return-tag protocol leaves (value, tag)
                // with the tag fresh after the call.
                let tag_dispatched = matches!(expr, Expr::Ternary { .. } | Expr::Call { .. })
                    && is_tag_carrying_read(expr, &self.ctx.variables);
                let (l_tflt, l_tstr, l_tend) = if tag_dispatched {
                    let flt = self.ctx.next_label();
                    let s2 = self.ctx.next_label();
                    let end = self.ctx.next_label();
                    if matches!(self.arch, Architecture::X64) {
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                        self.output.push_str(&format!("    je {}\n", flt));
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_STRING));
                        self.output.push_str(&format!("    je {}\n", s2));
                    } else {
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                        self.output.push_str(&format!("    b.eq {}\n", flt));
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_STRING));
                        self.output.push_str(&format!("    b.eq {}\n", s2));
                    }
                    (Some(flt), Some(s2), Some(end))
                } else {
                    (None, None, None)
                };

                if unknown_dynamic {
                    let l_call_str = self.ctx.next_label();
                    let l_call_end = self.ctx.next_label();
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.emit_runtime_classify(self.os);
                    arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::Equal,
                        false,
                        &l_call_str,
                        false,
                    );
                    arch::emit_pop_temp(&mut self.output, self.arch);
                    let fmt_call_int_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_call_int_label));
                    self.emit_string_directive("%lld\\n");
                    self.output.push_str(".text\n");
                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_call_int_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    arch::emit_jump(&mut self.output, self.arch, &l_call_end);
                    self.output.push_str(&format!("{}:\n", l_call_str));
                    arch::emit_pop_temp(&mut self.output, self.arch);
                    let fmt_call_str_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_call_str_label));
                    self.emit_string_directive("%s\\n");
                    self.output.push_str(".text\n");
                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_call_str_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    self.output.push_str(&format!("{}:\n", l_call_end));
                    if let (Some(_), Some(_), Some(t_end)) = (&l_tflt, &l_tstr, &l_tend) {
                        // Tag-dispatched calls share the arms below: skip
                        // the static print, which follows for untagged
                        // shapes. (Early return would leave the arm
                        // labels undefined.)
                        arch::emit_jump(&mut self.output, self.arch, t_end);
                    } else {
                        self.output.push('\n');
                        return;
                    }
                }

                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                if is_str || is_flt {
                    // B4: floats print via str_from_float, so both take %s.
                    self.emit_string_directive("%s\\n");
                } else {
                    self.emit_string_directive("%lld\\n");
                }

                self.output.push_str(".text\n");

                if is_flt {
                    arch::emit_say_float(
                        &mut self.output,
                        self.arch,
                        &fmt_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else {
                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_label,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
                if let (Some(t_flt), Some(t_str), Some(t_end)) = (l_tflt, l_tstr, l_tend) {
                    arch::emit_jump(&mut self.output, self.arch, &t_end);
                    self.output.push_str(&format!("{}:\n", t_flt));
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    movq %rax, %xmm0\n");
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    fmov d0, x0\n");
                        }
                    }
                    let fmt_tflt = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_tflt));
                    self.emit_string_directive("%s\\n");
                    self.output.push_str(".text\n");
                    arch::emit_say_float(
                        &mut self.output,
                        self.arch,
                        &fmt_tflt,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    arch::emit_jump(&mut self.output, self.arch, &t_end);
                    self.output.push_str(&format!("{}:\n", t_str));
                    let fmt_tstr = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_tstr));
                    self.emit_string_directive("%s\\n");
                    self.output.push_str(".text\n");
                    arch::emit_say_acc(
                        &mut self.output,
                        self.arch,
                        &fmt_tstr,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    self.output.push_str(&format!("{}:\n", t_end));
                }
                self.output.push('\n');
            }
        }
    }
}
