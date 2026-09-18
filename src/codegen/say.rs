use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    escape_string, is_float_expr, is_map_expr, is_null_expr, is_string_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

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
                let mut exprs = Vec::new();
                let mut is_floats = Vec::new();

                for part in parts {
                    match part {
                        Expr::String(s) => {
                            format_str.push_str(&escape_string(s).replace('%', "%%"));
                        }
                        _ => {
                            if is_null_expr(part, &self.ctx.variables) {
                                format_str.push_str("null");
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
                                exprs.push(part);
                                is_floats.push(is_flt);
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
                            let fmt_label = self.ctx.next_string_label();
                            self.emit_rodata_section();
                            self.output.push_str(&format!("{}:\n", fmt_label));
                            self.emit_string_directive("%lld\\n");

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
