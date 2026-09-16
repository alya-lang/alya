use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    escape_string, is_array_expr, is_float_expr, is_map_expr, is_null_expr, is_string_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::{Architecture, OperatingSystem};

impl CodeGen {
    pub(crate) fn generate_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Null => {
                arch::emit_load_num(&mut self.output, self.arch, 0);
            }
            Expr::Number(n) => {
                if n.fract() != 0.0 {
                    arch::emit_load_float(&mut self.output, self.arch, *n);
                } else {
                    arch::emit_load_num(&mut self.output, self.arch, *n as i64);
                }
            }
            Expr::Float(n) => {
                arch::emit_load_float(&mut self.output, self.arch, *n);
            }
            Expr::String(s) => {
                let label = self.ctx.next_string_label();
                self.emit_rodata_section();
                self.output.push_str(&format!("{}:\n", label));
                self.emit_string_directive(&escape_string(s));
                self.output.push_str(".text\n");

                arch::emit_load_str_label(&mut self.output, self.arch, &label, self.os);
            }
            Expr::Identifier(name) => {
                if let Some(var_type) = self.ctx.variables.get(name).cloned() {
                    match var_type {
                        VarType::Number(offset)
                        | VarType::StringOffset(offset)
                        | VarType::Array(offset)
                        | VarType::Map(offset)
                        | VarType::Null(offset)
                        | VarType::Struct { offset, .. } => {
                            arch::emit_load_var(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset,
                            );
                        }
                        VarType::Float(offset) => match self.arch {
                            Architecture::ARM64 => {
                                arch::arm64::loads::emit_arm64_load_x29_offset(
                                    &mut self.output,
                                    "d0",
                                    offset,
                                    "x9",
                                );
                                self.output.push_str("    fmov x0, d0\n");
                            }
                            Architecture::X64 => {
                                self.output
                                    .push_str(&format!("    movsd -{}(%rbp), %xmm0\n", offset));
                                self.output.push_str("    movq %xmm0, %rax\n");
                            }
                            Architecture::X86 => {
                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    self.ctx.stack_offset,
                                );
                            }
                        },
                        VarType::StringLabel(label) => {
                            arch::emit_load_str_label(&mut self.output, self.arch, &label, self.os);
                        }
                    }
                } else if self.ctx.functions.contains(name) {
                    let mangled = if name.contains("::") {
                        name.replace("::", "__")
                    } else {
                        name.clone()
                    };
                    match self.arch {
                        Architecture::ARM64 => {
                            if matches!(self.os, OperatingSystem::MacOS) {
                                self.output
                                    .push_str(&format!("    adrp x0, fn_{}@PAGE\n", mangled));
                                self.output
                                    .push_str(&format!("    add x0, x0, fn_{}@PAGEOFF\n", mangled));
                            } else {
                                self.output
                                    .push_str(&format!("    adrp x0, fn_{}\n", mangled));
                                self.output
                                    .push_str(&format!("    add x0, x0, :lo12:fn_{}\n", mangled));
                            }
                        }
                        Architecture::X64 => {
                            self.output
                                .push_str(&format!("    leaq fn_{}(%rip), %rax\n", mangled));
                        }
                        Architecture::X86 => {
                            self.output
                                .push_str(&format!("    movl $fn_{}, %eax\n", mangled));
                        }
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                if *op == BinaryOp::And {
                    let false_label = self.ctx.next_label();
                    let end_label = self.ctx.next_label();

                    self.generate_condition_jump_if_false(left, &false_label);
                    self.generate_condition_jump_if_false(right, &false_label);
                    arch::emit_load_num(&mut self.output, self.arch, 1);
                    arch::emit_jump(&mut self.output, self.arch, &end_label);

                    self.output.push_str(&format!("{}:\n", false_label));
                    arch::emit_load_num(&mut self.output, self.arch, 0);

                    self.output.push_str(&format!("{}:\n", end_label));
                    return;
                }

                if *op == BinaryOp::Or {
                    let true_label = self.ctx.next_label();
                    let end_label = self.ctx.next_label();

                    self.generate_condition_jump_if_true(left, &true_label);
                    self.generate_condition_jump_if_true(right, &true_label);
                    arch::emit_load_num(&mut self.output, self.arch, 0);
                    arch::emit_jump(&mut self.output, self.arch, &end_label);

                    self.output.push_str(&format!("{}:\n", true_label));
                    arch::emit_load_num(&mut self.output, self.arch, 1);

                    self.output.push_str(&format!("{}:\n", end_label));
                    return;
                }

                if matches!(op, BinaryOp::Add)
                    && (is_string_expr(left, &self.ctx.variables)
                        || is_string_expr(right, &self.ctx.variables))
                {
                    self.generate_string_concat(left, right);
                    return;
                }

                let left_is_num = matches!(**left, Expr::Number(_) | Expr::Float(_));
                let right_is_num = matches!(**right, Expr::Number(_) | Expr::Float(_));

                if matches!(
                    op,
                    BinaryOp::Equal
                        | BinaryOp::NotEqual
                        | BinaryOp::Less
                        | BinaryOp::LessEqual
                        | BinaryOp::Greater
                        | BinaryOp::GreaterEqual
                ) && !left_is_num
                    && !right_is_num
                    && (is_string_expr(left, &self.ctx.variables)
                        || is_string_expr(right, &self.ctx.variables))
                {
                    self.generate_string_equality(left, right, *op);
                    return;
                }

                let left_is_float = is_float_expr(left, &self.ctx.variables);
                let right_is_float = is_float_expr(right, &self.ctx.variables);
                let is_float = left_is_float || right_is_float;

                if is_float {
                    if let Expr::Float(n) = &**right {
                        self.generate_expression(left);
                        if !left_is_float {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                        return;
                    }
                    if let Expr::Number(n) = &**right {
                        self.generate_expression(left);
                        if !left_is_float {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                        return;
                    }
                    if let Expr::Identifier(name) = &**right {
                        if let Some(&VarType::Float(offset)) = self.ctx.variables.get(name) {
                            self.generate_expression(left);
                            if !left_is_float {
                                arch::emit_int_to_float(&mut self.output, self.arch);
                            }
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                offset,
                                true,
                            );
                            arch::emit_float_binary_op_reg(&mut self.output, self.arch, *op);
                            return;
                        }
                    }
                } else {
                    if let Expr::Number(n) = &**right {
                        self.generate_expression(left);
                        arch::emit_binary_op_imm(&mut self.output, self.arch, *op, *n as i64);
                        return;
                    }
                    if let Expr::Identifier(name) = &**right {
                        if let Some(&VarType::Number(offset)) = self.ctx.variables.get(name) {
                            self.generate_expression(left);
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                offset,
                                false,
                            );
                            arch::emit_binary_op_reg(&mut self.output, self.arch, *op);
                            return;
                        }
                    }
                    let is_commutative = matches!(
                        op,
                        BinaryOp::Add
                            | BinaryOp::Multiply
                            | BinaryOp::Equal
                            | BinaryOp::NotEqual
                            | BinaryOp::BitAnd
                            | BinaryOp::BitOr
                            | BinaryOp::BitXor
                    );
                    if is_commutative {
                        if let Expr::Number(n) = &**left {
                            self.generate_expression(right);
                            arch::emit_binary_op_imm(&mut self.output, self.arch, *op, *n as i64);
                            return;
                        }
                        if let Expr::Identifier(name) = &**left {
                            if let Some(&VarType::Number(offset)) = self.ctx.variables.get(name) {
                                self.generate_expression(right);
                                arch::emit_load_var_to_scratch(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    false,
                                );
                                arch::emit_binary_op_reg(&mut self.output, self.arch, *op);
                                return;
                            }
                        }
                    }
                }

                if is_float {
                    self.generate_expression(left);
                    if !left_is_float {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    if matches!(self.arch, Architecture::X86) {
                        self.output
                            .push_str("    sub $8, %esp\n    movsd %xmm0, (%esp)\n");
                    } else {
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }

                    self.ctx.stack_offset += 8;
                    self.generate_expression(right);
                    if !right_is_float {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    self.ctx.stack_offset -= 8;

                    arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                } else {
                    self.generate_expression(left);
                    arch::emit_push_temp(&mut self.output, self.arch);

                    self.ctx.stack_offset += 8;
                    self.generate_expression(right);
                    self.ctx.stack_offset -= 8;
                    arch::emit_binary_op(&mut self.output, self.arch, *op);
                }
            }
            Expr::Unary { op, expr } => {
                let is_float = is_float_expr(expr, &self.ctx.variables);
                self.generate_expression(expr);
                if is_float {
                    arch::emit_float_unary_op(&mut self.output, self.arch, *op);
                } else {
                    arch::emit_unary_op(&mut self.output, self.arch, *op);
                }
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_label = self.ctx.next_label();
                let end_label = self.ctx.next_label();

                let is_flt = is_float_expr(then_branch, &self.ctx.variables)
                    || is_float_expr(else_branch, &self.ctx.variables);

                self.generate_condition_jump_if_false(condition, &else_label);

                self.generate_expression(then_branch);
                if is_flt && !is_float_expr(then_branch, &self.ctx.variables) {
                    arch::emit_int_to_float(&mut self.output, self.arch);
                }
                arch::emit_jump(&mut self.output, self.arch, &end_label);

                self.output.push_str(&format!("{}:\n", else_label));
                self.generate_expression(else_branch);
                if is_flt && !is_float_expr(else_branch, &self.ctx.variables) {
                    arch::emit_int_to_float(&mut self.output, self.arch);
                }

                self.output.push_str(&format!("{}:\n", end_label));
            }
            Expr::NullCoalesce { value, default } => {
                let end_label = self.ctx.next_label();
                let is_flt = is_float_expr(value, &self.ctx.variables)
                    || is_float_expr(default, &self.ctx.variables);

                if is_flt {
                    let default_label = self.ctx.next_label();
                    self.generate_expression(value);
                    if !is_float_expr(value, &self.ctx.variables) {
                        arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            BinaryOp::Equal,
                            false,
                            &default_label,
                        );
                        arch::emit_int_to_float(&mut self.output, self.arch);
                        arch::emit_jump(&mut self.output, self.arch, &end_label);
                    } else {
                        arch::emit_jump(&mut self.output, self.arch, &end_label);
                    }

                    self.output.push_str(&format!("{}:\n", default_label));
                    self.generate_expression(default);
                    if !is_float_expr(default, &self.ctx.variables) {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    self.output.push_str(&format!("{}:\n", end_label));
                } else {
                    self.generate_expression(value);
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::NotEqual,
                        false,
                        &end_label,
                    );
                    self.generate_expression(default);
                    self.output.push_str(&format!("{}:\n", end_label));
                }
            }

            Expr::Call { name, args } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = self
                    .ctx
                    .structs
                    .get(name)
                    .or_else(|| self.ctx.structs.get(bare))
                    .cloned()
                {
                    let desc_label = format!("alya_struct_desc_{}", bare);
                    arch::emit_struct_new(
                        &mut self.output,
                        self.arch,
                        &desc_label,
                        sdef.fields.len(),
                        self.ctx.stack_offset,
                        self.os,
                    );
                    arch::emit_push_temp(&mut self.output, self.arch);
                    let temp_offset: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    self.ctx.stack_offset += temp_offset;

                    for (i, fname) in sdef.fields.iter().enumerate() {
                        let arg = if i < args.len() {
                            &args[i]
                        } else if let Some(Some(def_val)) = sdef.defaults.get(i) {
                            def_val
                        } else {
                            &Expr::Number(0.0)
                        };
                        let is_flt = is_float_expr(arg, &self.ctx.variables);
                        let is_str = is_string_expr(arg, &self.ctx.variables);
                        let is_arr = is_array_expr(arg, &self.ctx.variables);
                        let is_map = is_map_expr(arg, &self.ctx.variables);
                        if is_str {
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", name, fname),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", fname),
                                VarType::StringOffset(0),
                            );
                        } else if is_flt {
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", name, fname),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                        } else if is_arr {
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", name, fname),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                        } else if is_map {
                            self.ctx.variables.insert(
                                format!("struct_field_map:{}.{}", name, fname),
                                VarType::Map(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                        }
                        self.generate_expression(arg);
                        if self.is_heap_expression(arg) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_struct_field_set_imm(&mut self.output, self.arch, i);
                    }

                    self.ctx.stack_offset -= temp_offset;
                    arch::emit_pop_temp(&mut self.output, self.arch);
                    return;
                }

                if (name == "len" || name == "length")
                    && args.len() == 1
                    && matches!(&args[0], Expr::Array(_))
                {
                    self.generate_expression(&args[0]);
                    arch::emit_array_len(&mut self.output, self.arch);
                    return;
                }

                if (name == "array_len" || name == "arr_len") && args.len() == 1 {
                    self.generate_expression(&args[0]);
                    arch::emit_array_len(&mut self.output, self.arch);
                    return;
                }

                if name == "push" && args.len() == 2 {
                    if is_string_expr(&args[1], &self.ctx.variables) {
                        if let Expr::Identifier(arr_name) = &args[0] {
                            self.ctx
                                .variables
                                .insert(format!("arr_is_str:{}", arr_name), VarType::Number(0));
                        }
                    }
                    self.generate_expression(&args[0]);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    let temp_offset: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    self.ctx.stack_offset += temp_offset;
                    self.generate_expression(&args[1]);
                    if self.is_heap_expression(&args[1]) {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    self.ctx.stack_offset -= temp_offset;
                    arch::emit_array_push(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    return;
                }

                if name == "pop" && args.len() == 1 {
                    self.generate_expression(&args[0]);
                    arch::emit_array_pop(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    return;
                }

                if (name == "float" || name == "to_float" || name == "parse_float")
                    && args.len() == 1
                {
                    self.generate_expression(&args[0]);
                    if is_string_expr(&args[0], &self.ctx.variables) {
                        arch::emit_call_str_to_float(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    } else if !is_float_expr(&args[0], &self.ctx.variables) {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    return;
                }

                if (name == "int" || name == "to_int" || name == "parse_int") && args.len() == 1 {
                    self.generate_expression(&args[0]);
                    if is_float_expr(&args[0], &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    } else {
                        arch::emit_call_str_to_int(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    return;
                }

                if name == "str" && args.len() == 1 {
                    if is_string_expr(&args[0], &self.ctx.variables) {
                        self.generate_expression(&args[0]);
                        return;
                    }
                    if is_null_expr(&args[0], &self.ctx.variables) {
                        self.generate_expression(&Expr::String("null".into()));
                        return;
                    }
                }

                if (name == "bit_and"
                    || name == "bit_or"
                    || name == "bit_xor"
                    || name == "bit_shl"
                    || name == "bit_shr")
                    && args.len() == 2
                {
                    if let Expr::Number(n) = &args[1] {
                        self.generate_expression(&args[0]);
                        arch::emit_bit_op_imm(&mut self.output, self.arch, name, *n as i64);
                        return;
                    }
                    if let Expr::Identifier(var_name) = &args[1] {
                        if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                            self.generate_expression(&args[0]);
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                offset,
                                false,
                            );
                            arch::emit_bit_op_reg(&mut self.output, self.arch, name);
                            return;
                        }
                    }
                    let is_bit_commutative =
                        matches!(name.as_str(), "bit_and" | "bit_or" | "bit_xor");
                    if is_bit_commutative {
                        if let Expr::Number(n) = &args[0] {
                            self.generate_expression(&args[1]);
                            arch::emit_bit_op_imm(&mut self.output, self.arch, name, *n as i64);
                            return;
                        }
                        if let Expr::Identifier(var_name) = &args[0] {
                            if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name)
                            {
                                self.generate_expression(&args[1]);
                                arch::emit_load_var_to_scratch(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    false,
                                );
                                arch::emit_bit_op_reg(&mut self.output, self.arch, name);
                                return;
                            }
                        }
                    }
                    self.generate_expression(&args[0]);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.generate_expression(&args[1]);
                    arch::emit_bit_op(&mut self.output, self.arch, name);
                    return;
                }

                if name == "bit_not" && args.len() == 1 {
                    self.generate_expression(&args[0]);
                    arch::emit_bit_not(&mut self.output, self.arch);
                    return;
                }

                if (name == "char_code" || name == "char_code_at" || name == "byte_at")
                    && args.len() == 2
                {
                    if let Expr::Identifier(idx_name) = &args[1] {
                        if let Some(&VarType::Number(idx_offset)) = self.ctx.variables.get(idx_name)
                        {
                            self.generate_expression(&args[0]);
                            match self.arch {
                                Architecture::ARM64 => {
                                    self.output.push_str("    mov x2, x0\n");
                                }
                                Architecture::X64 => {
                                    self.output.push_str("    mov %rax, %rdx\n");
                                }
                                Architecture::X86 => {
                                    self.output.push_str("    mov %eax, %edx\n");
                                }
                            }
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                idx_offset,
                                false,
                            );
                            if matches!(self.arch, Architecture::X64) {
                                self.output.push_str("    mov %rbx, %rcx\n");
                            } else if matches!(self.arch, Architecture::X86) {
                                self.output.push_str("    mov %ebx, %ecx\n");
                            }
                            let done_label = self.ctx.next_label();
                            arch::emit_char_code_at_direct(
                                &mut self.output,
                                self.arch,
                                &done_label,
                            );
                            return;
                        }
                    }
                    self.generate_expression(&args[0]);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.generate_expression(&args[1]);
                    let done_label = self.ctx.next_label();
                    arch::emit_char_code_at(&mut self.output, self.arch, &done_label);
                    return;
                }

                if (name == "ord" || name == "char_code") && args.len() == 1 {
                    if let Expr::Call {
                        name: inner_name,
                        args: inner_args,
                    } = &args[0]
                    {
                        if (inner_name == "char_at" || inner_name == "charAt")
                            && inner_args.len() == 2
                        {
                            if let Expr::Identifier(idx_name) = &inner_args[1] {
                                if let Some(&VarType::Number(idx_offset)) =
                                    self.ctx.variables.get(idx_name)
                                {
                                    self.generate_expression(&inner_args[0]);
                                    match self.arch {
                                        Architecture::ARM64 => {
                                            self.output.push_str("    mov x2, x0\n");
                                        }
                                        Architecture::X64 => {
                                            self.output.push_str("    mov %rax, %rdx\n");
                                        }
                                        Architecture::X86 => {
                                            self.output.push_str("    mov %eax, %edx\n");
                                        }
                                    }
                                    arch::emit_load_var_to_scratch(
                                        &mut self.output,
                                        self.arch,
                                        idx_offset,
                                        false,
                                    );
                                    if matches!(self.arch, Architecture::X64) {
                                        self.output.push_str("    mov %rbx, %rcx\n");
                                    } else if matches!(self.arch, Architecture::X86) {
                                        self.output.push_str("    mov %ebx, %ecx\n");
                                    }
                                    let done_label = self.ctx.next_label();
                                    arch::emit_char_code_at_direct(
                                        &mut self.output,
                                        self.arch,
                                        &done_label,
                                    );
                                    return;
                                }
                            }
                            self.generate_expression(&inner_args[0]);
                            arch::emit_push_temp(&mut self.output, self.arch);
                            self.generate_expression(&inner_args[1]);
                            let done_label = self.ctx.next_label();
                            arch::emit_char_code_at(&mut self.output, self.arch, &done_label);
                            return;
                        }
                    } else if let Expr::Index { array, index } = &args[0] {
                        if is_string_expr(array, &self.ctx.variables) {
                            if let Expr::Identifier(idx_name) = &**index {
                                if let Some(&VarType::Number(idx_offset)) =
                                    self.ctx.variables.get(idx_name)
                                {
                                    self.generate_expression(array);
                                    match self.arch {
                                        Architecture::ARM64 => {
                                            self.output.push_str("    mov x2, x0\n");
                                        }
                                        Architecture::X64 => {
                                            self.output.push_str("    mov %rax, %rdx\n");
                                        }
                                        Architecture::X86 => {
                                            self.output.push_str("    mov %eax, %edx\n");
                                        }
                                    }
                                    arch::emit_load_var_to_scratch(
                                        &mut self.output,
                                        self.arch,
                                        idx_offset,
                                        false,
                                    );
                                    if matches!(self.arch, Architecture::X64) {
                                        self.output.push_str("    mov %rbx, %rcx\n");
                                    } else if matches!(self.arch, Architecture::X86) {
                                        self.output.push_str("    mov %ebx, %ecx\n");
                                    }
                                    let done_label = self.ctx.next_label();
                                    arch::emit_char_code_at_direct(
                                        &mut self.output,
                                        self.arch,
                                        &done_label,
                                    );
                                    return;
                                }
                            }
                            self.generate_expression(array);
                            arch::emit_push_temp(&mut self.output, self.arch);
                            self.generate_expression(index);
                            let done_label = self.ctx.next_label();
                            arch::emit_char_code_at(&mut self.output, self.arch, &done_label);
                            return;
                        }
                    }
                }

                let mut resolved_name = name.clone();
                let mut actual_args = args.clone();

                // 1. Static struct method call: Point.new(args) -> Point__new(args)
                if let Some(Expr::Identifier(type_name)) = actual_args.first() {
                    let mangled = format!("{}__{}", type_name, name);
                    if self.ctx.structs.contains_key(type_name)
                        && !self.ctx.variables.contains_key(type_name)
                    {
                        resolved_name = mangled;
                        actual_args.remove(0);
                    }
                }

                // 2. Struct instance method call via UFCS: p.distance(...) -> Point__distance(p, ...)
                if let Some(first_arg) = actual_args.first() {
                    let struct_name_opt = match first_arg {
                        Expr::Identifier(var_name) => match self.ctx.variables.get(var_name) {
                            Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                            _ => None,
                        },
                        Expr::FieldAccess { field, .. }
                        | Expr::OptionalFieldAccess { field, .. } => {
                            self.ctx
                                .variables
                                .get(&format!("struct_field_struct:{}", field))
                                .and_then(|vt| {
                                    if let VarType::Struct { struct_name, .. } = vt {
                                        Some(struct_name.clone())
                                    } else {
                                        None
                                    }
                                })
                        }
                        _ => None,
                    };
                    if let Some(sname) = struct_name_opt {
                        let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                        let candidate1 = format!("{}__{}", sname, name);
                        let candidate2 = format!("{}__{}", bare_sname, name);
                        if self.ctx.functions.contains(&candidate1) {
                            resolved_name = candidate1;
                        } else if self.ctx.functions.contains(&candidate2) {
                            resolved_name = candidate2;
                        }
                    }
                }

                // 3. Direct namespace / mangled name: Point::create -> Point__create
                if resolved_name.contains("::") {
                    let mangled = resolved_name.replace("::", "__");
                    if self.ctx.functions.contains(&mangled) {
                        resolved_name = mangled;
                    }
                }

                let call_name_str = if (resolved_name == "substring" || resolved_name == "substr")
                    && actual_args.len() == 2
                {
                    actual_args.push(Expr::Number(-1.0));
                    "substring".to_string()
                } else if resolved_name == "substr" {
                    "substring".to_string()
                } else if resolved_name == "length" {
                    "len".to_string()
                } else if (resolved_name == "contains" || resolved_name == "has")
                    && actual_args.len() == 2
                    && is_map_expr(&actual_args[0], &self.ctx.variables)
                {
                    "has".to_string()
                } else {
                    resolved_name
                };
                let call_name = call_name_str.as_str();

                let initial_stack_offset = self.ctx.stack_offset;
                let word_size: i32 = match self.arch {
                    Architecture::ARM64 => 16,
                    Architecture::X86 => 4,
                    _ => 8,
                };
                match self.arch {
                    Architecture::X86 => {
                        for (idx, arg) in actual_args.iter().rev().enumerate() {
                            self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                            self.generate_expression(arg);
                            arch::emit_push_temp(&mut self.output, self.arch);
                        }
                    }
                    _ => {
                        for (idx, arg) in actual_args.iter().enumerate() {
                            self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                            self.generate_expression(arg);
                            arch::emit_push_temp(&mut self.output, self.arch);
                        }
                    }
                }
                self.ctx.stack_offset = initial_stack_offset;
                let is_extern = self.ctx.extern_functions.contains_key(call_name)
                    || self
                        .ctx
                        .extern_functions
                        .contains_key(call_name.rsplit("::").next().unwrap_or(call_name));

                if is_extern {
                    let extern_name = call_name.rsplit("::").next().unwrap_or(call_name);
                    let extern_name = extern_name.rsplit("__").next().unwrap_or(extern_name);
                    arch::emit_c_function_call(
                        &mut self.output,
                        self.arch,
                        extern_name,
                        actual_args.len(),
                        initial_stack_offset,
                        self.os,
                    );
                } else {
                    arch::emit_function_call(
                        &mut self.output,
                        self.arch,
                        call_name,
                        actual_args.len(),
                        initial_stack_offset,
                        self.os,
                    );
                }
            }
            Expr::StructInit { name, fields } => {
                let field_count = self
                    .ctx
                    .structs
                    .get(name)
                    .map(|s| s.fields.len())
                    .unwrap_or(fields.len());
                let desc_label = format!("alya_struct_desc_{}", name);
                arch::emit_struct_new(
                    &mut self.output,
                    self.arch,
                    &desc_label,
                    field_count,
                    self.ctx.stack_offset,
                    self.os,
                );
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = self
                    .ctx
                    .structs
                    .get(name)
                    .or_else(|| self.ctx.structs.get(bare))
                    .cloned()
                {
                    for (i, fname) in sdef.fields.iter().enumerate() {
                        let fval = if let Some((_, val)) = fields.iter().find(|(k, _)| k == fname) {
                            val
                        } else if let Some(Some(def_val)) = sdef.defaults.get(i) {
                            def_val
                        } else {
                            continue;
                        };
                        let is_str = is_string_expr(fval, &self.ctx.variables);
                        let is_flt = is_float_expr(fval, &self.ctx.variables);
                        let is_arr = is_array_expr(fval, &self.ctx.variables);
                        let is_map = is_map_expr(fval, &self.ctx.variables);
                        if is_str {
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", name, fname),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", fname),
                                VarType::StringOffset(0),
                            );
                        } else if is_flt {
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", name, fname),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                        } else if is_arr {
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", name, fname),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                        } else if is_map {
                            self.ctx.variables.insert(
                                format!("struct_field_map:{}.{}", name, fname),
                                VarType::Map(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                        }
                    }
                } else {
                    for (fname, fval) in fields {
                        let is_str = is_string_expr(fval, &self.ctx.variables);
                        let is_flt = is_float_expr(fval, &self.ctx.variables);
                        let is_arr = is_array_expr(fval, &self.ctx.variables);
                        let is_map = is_map_expr(fval, &self.ctx.variables);
                        if is_str {
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}.{}", name, fname),
                                VarType::StringOffset(0),
                            );
                            self.ctx.variables.insert(
                                format!("struct_field_str:{}", fname),
                                VarType::StringOffset(0),
                            );
                        } else if is_flt {
                            self.ctx.variables.insert(
                                format!("struct_field_flt:{}.{}", name, fname),
                                VarType::Float(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_flt:{}", fname), VarType::Float(0));
                        } else if is_arr {
                            self.ctx.variables.insert(
                                format!("struct_field_arr:{}.{}", name, fname),
                                VarType::Array(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_arr:{}", fname), VarType::Array(0));
                        } else if is_map {
                            self.ctx.variables.insert(
                                format!("struct_field_map:{}.{}", name, fname),
                                VarType::Map(0),
                            );
                            self.ctx
                                .variables
                                .insert(format!("struct_field_map:{}", fname), VarType::Map(0));
                        }
                    }
                }
                arch::emit_push_temp(&mut self.output, self.arch);
                let temp_offset: i32 = match self.arch {
                    Architecture::ARM64 => 16,
                    Architecture::X86 => 4,
                    _ => 8,
                };
                self.ctx.stack_offset += temp_offset;

                if let Some(sdef) = self
                    .ctx
                    .structs
                    .get(name)
                    .or_else(|| self.ctx.structs.get(bare))
                    .cloned()
                {
                    for (i, fname) in sdef.fields.iter().enumerate() {
                        let arg_expr =
                            if let Some((_, fval)) = fields.iter().find(|(k, _)| k == fname) {
                                fval
                            } else if let Some(Some(def_val)) = sdef.defaults.get(i) {
                                def_val
                            } else {
                                &Expr::Number(0.0)
                            };
                        self.generate_expression(arg_expr);
                        if self.is_heap_expression(arg_expr) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_struct_field_set_imm(&mut self.output, self.arch, i);
                    }
                } else {
                    for (i, (_, fval)) in fields.iter().enumerate() {
                        self.generate_expression(fval);
                        if self.is_heap_expression(fval) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_struct_field_set_imm(&mut self.output, self.arch, i);
                    }
                }

                self.ctx.stack_offset -= temp_offset;
                arch::emit_pop_temp(&mut self.output, self.arch);
            }
            Expr::FieldAccess { object, field } => {
                let field_idx = self.resolve_struct_field_index(object, field);
                self.generate_expression(object);
                arch::emit_struct_field_get(&mut self.output, self.arch, field_idx);
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
            Expr::Array(elements) => {
                arch::emit_array_new(
                    &mut self.output,
                    self.arch,
                    elements.len(),
                    self.ctx.stack_offset,
                    self.os,
                );
                arch::emit_push_temp(&mut self.output, self.arch);
                let temp_offset: i32 = match self.arch {
                    Architecture::ARM64 => 16,
                    Architecture::X86 => 4,
                    _ => 8,
                };
                self.ctx.stack_offset += temp_offset;

                for (i, elem) in elements.iter().enumerate() {
                    self.generate_expression(elem);
                    if self.is_heap_expression(elem) {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    arch::emit_array_set_imm(&mut self.output, self.arch, i);
                }

                self.ctx.stack_offset -= temp_offset;
                arch::emit_pop_temp(&mut self.output, self.arch);
            }
            Expr::Map(entries) => {
                arch::emit_function_call(
                    &mut self.output,
                    self.arch,
                    "map",
                    0,
                    self.ctx.stack_offset,
                    self.os,
                );
                if !entries.is_empty() {
                    arch::emit_allocate_var(
                        &mut self.output,
                        self.arch,
                        &mut self.ctx.stack_offset,
                    );
                    let saved_offset = self.ctx.stack_offset;

                    for (k, v) in entries {
                        self.generate_expression(k);
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        let k_offset = self.ctx.stack_offset;

                        self.generate_expression(v);
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        let v_offset = self.ctx.stack_offset;

                        match self.arch {
                            Architecture::X86 => {
                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    v_offset,
                                    self.ctx.stack_offset,
                                );
                                if self.is_heap_expression(v) {
                                    arch::emit_rc_retain(
                                        &mut self.output,
                                        self.arch,
                                        self.ctx.stack_offset,
                                        self.os,
                                    );
                                }
                                arch::emit_push_temp(&mut self.output, self.arch);

                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    k_offset,
                                    self.ctx.stack_offset,
                                );
                                arch::emit_push_temp(&mut self.output, self.arch);

                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    saved_offset,
                                    self.ctx.stack_offset,
                                );
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                            _ => {
                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    saved_offset,
                                    self.ctx.stack_offset,
                                );
                                arch::emit_push_temp(&mut self.output, self.arch);

                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    k_offset,
                                    self.ctx.stack_offset,
                                );
                                arch::emit_push_temp(&mut self.output, self.arch);

                                arch::emit_load_var(
                                    &mut self.output,
                                    self.arch,
                                    v_offset,
                                    self.ctx.stack_offset,
                                );
                                if self.is_heap_expression(v) {
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

                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "set",
                            3,
                            self.ctx.stack_offset,
                            self.os,
                        );

                        arch::emit_pop_temp(&mut self.output, self.arch);
                        arch::emit_pop_temp(&mut self.output, self.arch);
                        self.ctx.stack_offset -= match self.arch {
                            Architecture::ARM64 => 32,
                            Architecture::X86 => 8,
                            _ => 16,
                        };
                    }

                    arch::emit_pop_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset -= match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                }
            }
            Expr::Index { array, index } => {
                if is_map_expr(array, &self.ctx.variables)
                    || is_string_expr(index, &self.ctx.variables)
                    || matches!(**index, Expr::String(_))
                {
                    let actual_args = [array.as_ref(), index.as_ref()];
                    let initial_stack_offset = self.ctx.stack_offset;
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    match self.arch {
                        Architecture::X86 => {
                            for (idx, arg) in actual_args.iter().rev().enumerate() {
                                self.ctx.stack_offset =
                                    initial_stack_offset + (idx as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                        }
                        _ => {
                            for (idx, arg) in actual_args.iter().enumerate() {
                                self.ctx.stack_offset =
                                    initial_stack_offset + (idx as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                        }
                    }
                    self.ctx.stack_offset = initial_stack_offset;
                    arch::emit_function_call(
                        &mut self.output,
                        self.arch,
                        "get",
                        2,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else if is_string_expr(array, &self.ctx.variables) {
                    let actual_args = [array.as_ref(), index.as_ref()];
                    let initial_stack_offset = self.ctx.stack_offset;
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    match self.arch {
                        Architecture::X86 => {
                            for (idx, arg) in actual_args.iter().rev().enumerate() {
                                self.ctx.stack_offset =
                                    initial_stack_offset + (idx as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                        }
                        _ => {
                            for (idx, arg) in actual_args.iter().enumerate() {
                                self.ctx.stack_offset =
                                    initial_stack_offset + (idx as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                        }
                    }
                    self.ctx.stack_offset = initial_stack_offset;
                    arch::emit_function_call(
                        &mut self.output,
                        self.arch,
                        "char_at",
                        2,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else {
                    let initial_stack_offset = self.ctx.stack_offset;
                    let temp_offset: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    self.generate_expression(array);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset += temp_offset;

                    self.generate_expression(index);
                    arch::emit_array_get(&mut self.output, self.arch);
                    self.ctx.stack_offset = initial_stack_offset;
                }
            }
            Expr::InterpolatedString(parts) => {
                if parts.is_empty() {
                    self.generate_expression(&Expr::String(String::new()));
                } else {
                    let mut iter = parts.iter();
                    let first = iter.next().unwrap();
                    let mut acc = if is_string_expr(first, &self.ctx.variables) {
                        first.clone()
                    } else {
                        Expr::Call {
                            name: "str".into(),
                            args: vec![first.clone()],
                        }
                    };
                    for next in iter {
                        let next_expr = if is_string_expr(next, &self.ctx.variables) {
                            next.clone()
                        } else {
                            Expr::Call {
                                name: "str".into(),
                                args: vec![next.clone()],
                            }
                        };
                        acc = Expr::Binary {
                            left: Box::new(acc),
                            op: BinaryOp::Add,
                            right: Box::new(next_expr),
                        };
                    }
                    self.generate_expression(&acc);
                }
            }
            Expr::OptionalFieldAccess { object, field } => {
                let null_label = self.ctx.next_label();
                let end_label = self.ctx.next_label();

                let field_idx = self.resolve_struct_field_index(object, field);

                self.generate_expression(object);
                arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                arch::emit_cond_jump(
                    &mut self.output,
                    self.arch,
                    BinaryOp::Equal,
                    false,
                    &null_label,
                );

                arch::emit_struct_field_get(&mut self.output, self.arch, field_idx);
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq %rax, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    fmov d0, x0\n");
                    }
                    Architecture::X86 => {}
                }
                arch::emit_jump(&mut self.output, self.arch, &end_label);

                self.output.push_str(&format!("{}:\n", null_label));
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq $0, %rax\n");
                        self.output.push_str("    xorpd %xmm0, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    mov x0, #0\n");
                        self.output.push_str("    fmov d0, xzr\n");
                    }
                    Architecture::X86 => {
                        self.output.push_str("    movl $0, %eax\n");
                    }
                }
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Expr::OptionalIndex { array, index } => {
                let null_label = self.ctx.next_label();
                let end_label = self.ctx.next_label();

                self.generate_expression(array);
                arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                arch::emit_cond_jump(
                    &mut self.output,
                    self.arch,
                    BinaryOp::Equal,
                    false,
                    &null_label,
                );

                self.generate_expression(&Expr::Index {
                    array: array.clone(),
                    index: index.clone(),
                });
                arch::emit_jump(&mut self.output, self.arch, &end_label);

                self.output.push_str(&format!("{}:\n", null_label));
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq $0, %rax\n");
                        self.output.push_str("    xorpd %xmm0, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    mov x0, #0\n");
                        self.output.push_str("    fmov d0, xzr\n");
                    }
                    Architecture::X86 => {
                        self.output.push_str("    movl $0, %eax\n");
                    }
                }
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Expr::OptionalCall { callee, args } => {
                let null_label = self.ctx.next_label();
                let end_label = self.ctx.next_label();

                if let Some(target) = args.first() {
                    self.generate_expression(target);
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::Equal,
                        false,
                        &null_label,
                    );
                }

                self.generate_expression(&Expr::Call {
                    name: callee.clone(),
                    args: args.clone(),
                });
                arch::emit_jump(&mut self.output, self.arch, &end_label);

                self.output.push_str(&format!("{}:\n", null_label));
                match self.arch {
                    Architecture::X64 => {
                        self.output.push_str("    movq $0, %rax\n");
                        self.output.push_str("    xorpd %xmm0, %xmm0\n");
                    }
                    Architecture::ARM64 => {
                        self.output.push_str("    mov x0, #0\n");
                        self.output.push_str("    fmov d0, xzr\n");
                    }
                    Architecture::X86 => {
                        self.output.push_str("    movl $0, %eax\n");
                    }
                }
                self.output.push_str(&format!("{}:\n", end_label));
            }
        }
    }

    pub(crate) fn generate_string_concat(&mut self, left: &Expr, right: &Expr) {
        if is_string_expr(left, &self.ctx.variables) {
            self.generate_expression(left);
        } else {
            self.generate_expression(&Expr::Call {
                name: "str".into(),
                args: vec![left.clone()],
            });
        }
        arch::emit_push_temp(&mut self.output, self.arch);
        self.ctx.stack_offset += 8;

        if is_string_expr(right, &self.ctx.variables) {
            self.generate_expression(right);
        } else {
            self.generate_expression(&Expr::Call {
                name: "str".into(),
                args: vec![right.clone()],
            });
        }
        self.ctx.stack_offset -= 8;
        arch::emit_string_concat_call(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
    }

    pub(crate) fn generate_string_equality(&mut self, left: &Expr, right: &Expr, op: BinaryOp) {
        self.generate_expression(left);
        arch::emit_push_temp(&mut self.output, self.arch);
        self.ctx.stack_offset += 8;

        self.generate_expression(right);
        self.ctx.stack_offset -= 8;
        arch::emit_string_equality_call(
            &mut self.output,
            self.arch,
            op,
            self.ctx.stack_offset,
            self.os,
        );
    }

    fn resolve_struct_field_index(&self, object: &Expr, field: &str) -> usize {
        let mut field_idx = 0;
        let mut struct_found = false;

        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            _ => object,
        };

        if let Expr::Identifier(obj_name) = base_obj {
            if let Some(VarType::Struct { struct_name, .. }) = self.ctx.variables.get(obj_name) {
                let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = self
                    .ctx
                    .structs
                    .get(struct_name)
                    .or_else(|| self.ctx.structs.get(bare))
                {
                    if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                        field_idx = idx;
                        struct_found = true;
                    }
                }
            }
        } else if let Expr::FieldAccess {
            field: inner_field, ..
        }
        | Expr::OptionalFieldAccess {
            field: inner_field, ..
        } = base_obj
        {
            if let Some(VarType::Struct { struct_name, .. }) = self
                .ctx
                .variables
                .get(&format!("struct_field_struct:{}", inner_field))
            {
                let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = self
                    .ctx
                    .structs
                    .get(struct_name)
                    .or_else(|| self.ctx.structs.get(bare))
                {
                    if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                        field_idx = idx;
                        struct_found = true;
                    }
                }
            }
        }

        if !struct_found {
            for sdef in self.ctx.structs.values() {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    field_idx = idx;
                    break;
                }
            }
        }

        field_idx
    }
}
