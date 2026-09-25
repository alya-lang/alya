use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    escape_string, is_array_expr, is_float_expr, is_map_expr, is_map_read_index, is_null_expr,
    is_string_array, is_string_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
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
                                format_str.push_str(&format!("%{}lld", spec));
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
                                format_str.push_str("%lld");
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
                                } else {
                                    let is_flt = is_float_expr(part, &self.ctx.variables);
                                    let is_str = is_string_expr(part, &self.ctx.variables);
                                    if is_str {
                                        format_str.push_str("%s");
                                    } else if is_flt {
                                        format_str.push_str("%g");
                                    } else {
                                        format_str.push_str("%lld");
                                    }
                                    exprs.push(part.clone());
                                    is_floats.push(is_flt);
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
                    Architecture::X86 => 4,
                    _ => 8,
                };
                match self.arch {
                    Architecture::X86 => {
                        for (idx, expr) in exprs.iter().rev().enumerate() {
                            self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                            self.generate_expression(expr);
                            arch::emit_push_temp(&mut self.output, self.arch);
                        }
                    }
                    _ => {
                        for (idx, expr) in exprs.iter().enumerate() {
                            self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                            self.generate_expression(expr);
                            arch::emit_push_temp(&mut self.output, self.arch);
                        }
                    }
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
                let is_flt = is_float_expr(expr, &self.ctx.variables);
                self.generate_expression(expr);

                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                if is_flt {
                    self.emit_string_directive("%g\\n");
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
                                self.output.push('\n');
                                return;
                            }
                            // Statically-unknown dynamics (e.g. map reads with
                            // variable keys) are recorded as Number and would
                            // print string pointers as integers. Classify at
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
                            arch::emit_cmp_imm(&mut self.output, self.arch, 3);
                            arch::emit_cond_jump(
                                &mut self.output,
                                self.arch,
                                BinaryOp::Equal,
                                false,
                                &l_dyn_str,
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
                            self.emit_string_directive("%g\\n");
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
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                            arch::emit_print_map(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
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
                            self.output.push_str("    movq (%rax), %rax\n");
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
                self.emit_string_directive("%g\\n");
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
                if n.fract() != 0.0 {
                    let fmt_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_label));
                    self.emit_string_directive("%g\\n");
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
                } else {
                    let fmt_label = self.ctx.next_string_label();
                    self.emit_rodata_section();
                    self.output.push_str(&format!("{}:\n", fmt_label));
                    self.emit_string_directive("%lld\\n");
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
                        self.ctx
                            .functions
                            .iter()
                            .find(|f| f.ends_with("__to_string"))
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
                    self.generate_expression(expr);
                    arch::emit_print_map(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
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
                        // (x64: %edx, arm64: w1). Tagged values dispatch
                        // directly; unknown falls through to the legacy
                        // pointer-range classifier below. x86 stays untagged.
                        // Tags: 0 unknown, 1 int, 2 float, 3 string,
                        // 4 array, 5 map.
                        // NOTE: only map-routed reads (fn_get) carry a tag.
                        // Direct array loads leave the tag register holding
                        // the index, so they must skip tag dispatch.
                        let (l_tag_flt, l_tag_str2, l_tag_arr, l_tag_map, l_tag_int) =
                            if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && is_map_read_index(expr, &self.ctx.variables)
                            {
                                let flt = self.ctx.next_label();
                                let s2 = self.ctx.next_label();
                                let arr = self.ctx.next_label();
                                let mp = self.ctx.next_label();
                                let it = self.ctx.next_label();
                                if matches!(self.arch, Architecture::X64) {
                                    self.output.push_str("    cmpl $2, %edx\n");
                                    self.output.push_str(&format!("    je {}\n", flt));
                                    self.output.push_str("    cmpl $3, %edx\n");
                                    self.output.push_str(&format!("    je {}\n", s2));
                                    self.output.push_str("    cmpl $4, %edx\n");
                                    self.output.push_str(&format!("    je {}\n", arr));
                                    self.output.push_str("    cmpl $5, %edx\n");
                                    self.output.push_str(&format!("    je {}\n", mp));
                                    self.output.push_str("    cmpl $1, %edx\n");
                                    self.output.push_str(&format!("    je {}\n", it));
                                } else {
                                    self.output.push_str("    cmp w1, #2\n");
                                    self.output.push_str(&format!("    b.eq {}\n", flt));
                                    self.output.push_str("    cmp w1, #3\n");
                                    self.output.push_str(&format!("    b.eq {}\n", s2));
                                    self.output.push_str("    cmp w1, #4\n");
                                    self.output.push_str(&format!("    b.eq {}\n", arr));
                                    self.output.push_str("    cmp w1, #5\n");
                                    self.output.push_str(&format!("    b.eq {}\n", mp));
                                    self.output.push_str("    cmp w1, #1\n");
                                    self.output.push_str(&format!("    b.eq {}\n", it));
                                }
                                (Some(flt), Some(s2), Some(arr), Some(mp), Some(it))
                            } else {
                                (None, None, None, None, None)
                            };
                        self.emit_runtime_classify(self.os);
                        arch::emit_cmp_imm(&mut self.output, self.arch, 3);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            BinaryOp::Equal,
                            false,
                            &l_idx_str,
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
                        // Tagged fast paths (x64 only). Each pops the saved
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
                            self.emit_string_directive("%g\\n");
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
                self.generate_expression(expr);

                let fmt_label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", fmt_label));
                if is_str {
                    self.emit_string_directive("%s\\n");
                } else if is_flt {
                    self.emit_string_directive("%g\\n");
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
        }
    }
}
