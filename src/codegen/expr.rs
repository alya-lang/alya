use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    escape_string, is_array_expr, is_float_expr, is_map_expr, is_null_expr, is_number_expr,
    is_string_expr,
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
                    let has_local_offset = match var_type {
                        VarType::Number(offset)
                        | VarType::StringOffset(offset)
                        | VarType::Array(offset)
                        | VarType::Map(offset)
                        | VarType::Null(offset)
                        | VarType::Float(offset)
                        | VarType::Struct { offset, .. } => offset != 0,
                        _ => false,
                    };
                    if has_local_offset {
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
                                return;
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
                                    return;
                                }
                                Architecture::X64 => {
                                    self.output
                                        .push_str(&format!("    movsd -{}(%rbp), %xmm0\n", offset));
                                    self.output.push_str("    movq %xmm0, %rax\n");
                                    return;
                                }
                                Architecture::X86 => {
                                    arch::emit_load_var(
                                        &mut self.output,
                                        self.arch,
                                        offset,
                                        self.ctx.stack_offset,
                                    );
                                    return;
                                }
                            },
                            _ => {}
                        }
                    }
                }
                if let Some((symbol, _)) = self.ctx.globals.get(name) {
                    arch::emit_load_global(&mut self.output, self.arch, symbol, self.os);
                    return;
                }
                if let Some(var_type) = self.ctx.variables.get(name).cloned() {
                    match var_type {
                        VarType::Number(offset)
                        | VarType::StringOffset(offset)
                        | VarType::Array(offset)
                        | VarType::Map(offset)
                        | VarType::Null(offset)
                        | VarType::Struct { offset, .. }
                        | VarType::Interface { offset, .. } => {
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

                if matches!(op, BinaryOp::In | BinaryOp::NotIn) {
                    self.generate_expression(left);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset += 8;

                    self.generate_expression(right);
                    self.ctx.stack_offset -= 8;

                    arch::emit_in_call(
                        &mut self.output,
                        self.arch,
                        *op,
                        self.ctx.stack_offset,
                        self.os,
                    );
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
                        let is_weak = sdef
                            .field_types
                            .get(i)
                            .and_then(|t| t.as_deref())
                            .map_or(false, |t| t.starts_with("weak ") || t == "weak");
                        self.generate_expression(arg);
                        if !is_weak && self.is_heap_expression(arg) {
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

                let is_struct_receiver = args.first().and_then(|a| self.get_expr_struct_name(a)).is_some_and(|sname| {
                    let bare = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    self.ctx.functions.contains(&format!("{}__{}", sname, name))
                        || self.ctx.functions.contains(&format!("{}__{}", bare, name))
                        || self.ctx.functions.iter().any(|f| f.ends_with(&format!("{}__{}", bare, name)))
                });

                if name == "push" && args.len() == 2 && !is_struct_receiver {
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

                if name == "pop" && args.len() == 1 && !is_struct_receiver {
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
                    if is_float_expr(&args[0], &self.ctx.variables) {
                        let initial_stack_offset = self.ctx.stack_offset;
                        self.generate_expression(&args[0]);
                        arch::emit_push_temp(&mut self.output, self.arch);
                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "str_from_float",
                            1,
                            initial_stack_offset,
                            self.os,
                        );
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

                // 1. Static struct method call: Point.new(args) -> Point__new(args) or Module.Struct.new(args)
                let static_type_info = match actual_args.first() {
                    Some(Expr::Identifier(type_name)) => Some((None, type_name.clone())),
                    Some(Expr::FieldAccess { object, field }) => {
                        if let Expr::Identifier(mod_name) = &**object {
                            Some((Some(mod_name.clone()), field.clone()))
                        } else {
                            None
                        }
                    }
                    Some(Expr::Index { array, .. }) => {
                        match &**array {
                            Expr::Identifier(type_name) => Some((None, type_name.clone())),
                            Expr::FieldAccess { object, field } => {
                                if let Expr::Identifier(mod_name) = &**object {
                                    Some((Some(mod_name.clone()), field.clone()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };

                if let Some((mod_opt, type_name)) = static_type_info {
                    let mangled1 = format!("{}__{}", type_name, name);
                    let mangled_single = format!("{}_{}", type_name, name);
                    let mangled2 = if let Some(ref m) = mod_opt {
                        format!("{}__{}__{}", m, type_name, name)
                    } else {
                        mangled1.clone()
                    };
                    let mangled3 = if let Some(ref m) = mod_opt {
                        format!("{}::{}__{}", m, type_name, name)
                    } else {
                        mangled1.clone()
                    };
                    let is_struct = self.ctx.structs.contains_key(&type_name)
                        || mod_opt.as_ref().map_or(false, |m| self.ctx.structs.contains_key(&format!("{}::{}", m, type_name)));
                    let is_var = self.ctx.variables.contains_key(&type_name);
                    let is_mod_var = mod_opt.as_ref().map_or(false, |m| self.ctx.variables.contains_key(m));

                    let valid_target = if mod_opt.is_some() {
                        is_struct && !is_mod_var
                    } else {
                        (is_struct
                            || self.ctx.functions.contains(&mangled1)
                            || self.ctx.functions.contains(&mangled_single)
                            || self.ctx.functions.contains(&mangled2)
                            || self.ctx.functions.contains(&mangled3)
                            || self.ctx.functions.iter().any(|f| f.ends_with(&format!("__{}", mangled1)) || f.ends_with(&format!("::{}", mangled1))))
                            && !is_var
                    };

                    if valid_target
                    {
                        if self.ctx.functions.contains(&mangled2) {
                            resolved_name = mangled2;
                        } else if self.ctx.functions.contains(&mangled3) {
                            resolved_name = mangled3;
                        } else if self.ctx.functions.contains(&mangled1) {
                            resolved_name = mangled1;
                        } else if self.ctx.functions.contains(&mangled_single) {
                            resolved_name = mangled_single;
                        } else if let Some(matched) = self.ctx.functions.iter().find(|f| {
                            f.ends_with(&format!("__{}", mangled1)) || f.ends_with(&format!("::{}", mangled1))
                        }) {
                            resolved_name = matched.clone();
                        } else {
                            resolved_name = mangled1;
                        }
                        actual_args.remove(0);
                    }
                }

                // 1b. Interface instance dynamic method dispatch: s.area()
                if let Some(first_arg) = actual_args.first() {
                    if let Some(iname) = self.get_expr_interface_name(first_arg) {
                        let flattened = crate::codegen::get_interface_flattened_methods(&iname, &self.ctx.interfaces);
                        if let Some((method_idx, _)) = flattened.iter().enumerate().find(|(_, m)| m.name == *name) {
                            let initial_stack_offset = self.ctx.stack_offset;
                            let word_size: i32 = match self.arch {
                                Architecture::ARM64 => 16,
                                Architecture::X86 => 4,
                                _ => 8,
                            };

                            // Evaluate receiver: load concrete instance data_ptr (offset 0 of fat pointer)
                            self.generate_expression(first_arg);
                            self.output.push_str("    movq (%rax), %rax\n");
                            arch::emit_push_temp(&mut self.output, self.arch);

                            // Evaluate remaining arguments
                            for (idx, arg) in actual_args.iter().skip(1).enumerate() {
                                self.ctx.stack_offset = initial_stack_offset + ((idx + 1) as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                            self.ctx.stack_offset = initial_stack_offset;

                            // Load method function pointer from vtable:
                            // 1. Load fat pointer again
                            self.generate_expression(first_arg);
                            // 2. Load vtable pointer: 8(%rax)
                            self.output.push_str("    movq 8(%rax), %r11\n");
                            // 3. Load function pointer: ((method_idx + 1) * 8)(%r11)
                            self.output.push_str(&format!("    movq {}(%r11), %r11\n", (method_idx + 1) * 8));

                            // 4. Call function pointer
                            arch::x64::control::emit_call_target(
                                &mut self.output,
                                "*%r11",
                                actual_args.len(),
                                initial_stack_offset,
                                self.os,
                            );

                            let is_flt_ret = flattened[method_idx]
                                .return_type
                                .as_deref()
                                .map_or(false, |rt| rt == "float" || rt == "f64")
                                || self.ctx.variables.contains_key(&format!("fn_ret_flt:{}", name));
                            if is_flt_ret && matches!(self.arch, Architecture::X64) {
                                self.output.push_str("    movq %xmm0, %rax\n");
                            }
                            return;
                        }
                    }
                }

                // 2. Struct instance method call via UFCS: p.distance(...) -> Point__distance(p, ...)
                if let Some(first_arg) = actual_args.first() {
                    let struct_name_opt = self.get_expr_struct_name(first_arg);
                    if let Some(sname) = struct_name_opt {
                        let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                        let candidate1 = format!("{}__{}", sname, name);
                        let candidate2 = format!("{}__{}", bare_sname, name);
                        let suffix1 = format!("__{}", candidate1);
                        let suffix2 = format!("__{}", candidate2);
                        let suffix3 = format!("::{}", candidate1);
                        let suffix4 = format!("::{}", candidate2);
                        if self.ctx.functions.contains(&candidate1) {
                            resolved_name = candidate1;
                        } else if self.ctx.functions.contains(&candidate2) {
                            resolved_name = candidate2;
                        } else if let Some(matched) = self.ctx.functions.iter().find(|f| {
                            f.ends_with(&suffix1)
                                || f.ends_with(&suffix2)
                                || f.ends_with(&suffix3)
                                || f.ends_with(&suffix4)
                        }) {
                            resolved_name = matched.clone();
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
                            let param_idx = actual_args.len() - 1 - idx;
                            let coerce_vtable = if self.get_expr_interface_name(arg).is_some() {
                                None
                            } else if let Some(sname) = self.get_expr_struct_name(arg) {
                                let expected_iface = self
                                    .ctx
                                    .variables
                                    .get(&format!("fn_param_interface:{}:{}", call_name, param_idx))
                                    .or_else(|| {
                                        let bare = call_name.rsplit("::").next().unwrap_or(call_name);
                                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                                        self.ctx.variables.get(&format!("fn_param_interface:{}:{}", bare, param_idx))
                                    })
                                    .and_then(|vt| match vt {
                                        VarType::Interface { interface_name, .. } => Some(interface_name.clone()),
                                        _ => None,
                                    });
                                if let Some(iname) = expected_iface {
                                    let bare_s = sname.rsplit("::").next().unwrap_or(&sname);
                                    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                                    let bare_i = iname.rsplit("::").next().unwrap_or(&iname);
                                    let bare_i = bare_i.rsplit("__").next().unwrap_or(bare_i);

                                    let vtable = self
                                        .ctx
                                        .vtables
                                        .get(&(bare_s.to_string(), bare_i.to_string()))
                                        .or_else(|| self.ctx.vtables.get(&(sname.clone(), iname.clone())))
                                        .cloned()
                                        .unwrap_or_else(|| format!("alya_vtable_{}_{}", bare_s, bare_i));
                                    Some(vtable)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                            self.generate_expression(arg);
                            if let Some(vtable_label) = coerce_vtable {
                                arch::emit_fat_ptr_new(&mut self.output, self.arch, &vtable_label, self.ctx.stack_offset, self.os);
                            }
                            arch::emit_push_temp(&mut self.output, self.arch);
                        }
                    }
                    _ => {
                        for (param_idx, arg) in actual_args.iter().enumerate() {
                            let coerce_vtable = if self.get_expr_interface_name(arg).is_some() {
                                None
                            } else if let Some(sname) = self.get_expr_struct_name(arg) {
                                let expected_iface = self
                                    .ctx
                                    .variables
                                    .get(&format!("fn_param_interface:{}:{}", call_name, param_idx))
                                    .or_else(|| {
                                        let bare = call_name.rsplit("::").next().unwrap_or(call_name);
                                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                                        self.ctx.variables.get(&format!("fn_param_interface:{}:{}", bare, param_idx))
                                    })
                                    .and_then(|vt| match vt {
                                        VarType::Interface { interface_name, .. } => Some(interface_name.clone()),
                                        _ => None,
                                    });
                                if let Some(iname) = expected_iface {
                                    let bare_s = sname.rsplit("::").next().unwrap_or(&sname);
                                    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                                    let bare_i = iname.rsplit("::").next().unwrap_or(&iname);
                                    let bare_i = bare_i.rsplit("__").next().unwrap_or(bare_i);

                                    let vtable = self
                                        .ctx
                                        .vtables
                                        .get(&(bare_s.to_string(), bare_i.to_string()))
                                        .or_else(|| self.ctx.vtables.get(&(sname.clone(), iname.clone())))
                                        .cloned()
                                        .unwrap_or_else(|| format!("alya_vtable_{}_{}", bare_s, bare_i));
                                    Some(vtable)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            self.ctx.stack_offset = initial_stack_offset + (param_idx as i32 * word_size);
                            self.generate_expression(arg);
                            if let Some(vtable_label) = coerce_vtable {
                                arch::emit_fat_ptr_new(&mut self.output, self.arch, &vtable_label, self.ctx.stack_offset, self.os);
                            }
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

                let var_offset = if !self.ctx.functions.contains(call_name) && !is_extern {
                    match self.ctx.variables.get(call_name) {
                        Some(VarType::Number(off))
                        | Some(VarType::Float(off))
                        | Some(VarType::StringOffset(off))
                        | Some(VarType::Array(off))
                        | Some(VarType::Map(off))
                        | Some(VarType::Null(off))
                        | Some(VarType::Struct { offset: off, .. }) => Some(*off),
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some(offset) = var_offset {
                    arch::emit_indirect_function_call(
                        &mut self.output,
                        self.arch,
                        offset,
                        actual_args.len(),
                        initial_stack_offset,
                        self.os,
                    );
                } else if is_extern {
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
                        let is_weak = sdef
                            .field_types
                            .get(i)
                            .and_then(|t| t.as_deref())
                            .map_or(false, |t| t.starts_with("weak ") || t == "weak");
                        self.generate_expression(arg_expr);
                        if !is_weak && self.is_heap_expression(arg_expr) {
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
                let is_weak = self.is_struct_field_weak(object, field);
                self.generate_expression(object);
                arch::emit_struct_field_get(&mut self.output, self.arch, field_idx);
                if is_weak {
                    let lbl = self.ctx.next_label();
                    arch::emit_weak_check(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                        &lbl,
                    );
                }
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
                let is_weak = self.is_struct_field_weak(object, field);

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
                if is_weak {
                    let lbl = self.ctx.next_label();
                    arch::emit_weak_check(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                        &lbl,
                    );
                }
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

                let is_callee_var =
                    self.ctx.variables.contains_key(callee) && !self.ctx.functions.contains(callee);

                if is_callee_var {
                    self.generate_expression(&Expr::Identifier(callee.clone()));
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::Equal,
                        false,
                        &null_label,
                    );
                } else if let Some(target) = args.first() {
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
            Expr::TypeCheck {
                expr,
                target,
                negated,
            } => {
                self.generate_type_check(expr, target, *negated);
            }
            Expr::Cast { expr, .. } => {
                self.generate_expression(expr);
            }
        }
    }

    fn generate_type_check(&mut self, expr: &Expr, target: &str, negated: bool) {
        let t = target.to_lowercase();
        match t.as_str() {
            "null" | "nil" => {
                self.generate_expression(&Expr::Binary {
                    left: Box::new(expr.clone()),
                    op: if negated {
                        BinaryOp::NotEqual
                    } else {
                        BinaryOp::Equal
                    },
                    right: Box::new(Expr::Null),
                });
            }
            "string" | "str" => {
                let is_str = is_string_expr(expr, &self.ctx.variables);
                let result = if is_str {
                    if negated {
                        0
                    } else {
                        1
                    }
                } else {
                    if negated {
                        1
                    } else {
                        0
                    }
                };
                arch::emit_load_num(&mut self.output, self.arch, result);
            }
            "array" | "list" => {
                let is_arr = is_array_expr(expr, &self.ctx.variables);
                let is_def_non = is_string_expr(expr, &self.ctx.variables)
                    || is_map_expr(expr, &self.ctx.variables)
                    || is_float_expr(expr, &self.ctx.variables)
                    || is_null_expr(expr, &self.ctx.variables)
                    || is_number_expr(expr, &self.ctx.variables);
                if is_arr {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 0 } else { 1 });
                } else if is_def_non {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 1 } else { 0 });
                } else {
                    self.emit_runtime_tag_check(expr, 0x5A110001, negated);
                }
            }
            "map" | "dict" => {
                let is_map = is_map_expr(expr, &self.ctx.variables);
                let is_def_non = is_string_expr(expr, &self.ctx.variables)
                    || is_array_expr(expr, &self.ctx.variables)
                    || is_float_expr(expr, &self.ctx.variables)
                    || is_null_expr(expr, &self.ctx.variables)
                    || is_number_expr(expr, &self.ctx.variables);
                if is_map {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 0 } else { 1 });
                } else if is_def_non {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 1 } else { 0 });
                } else {
                    self.emit_runtime_tag_check(expr, 0x5A110002, negated);
                }
            }
            "float" => {
                let is_flt = is_float_expr(expr, &self.ctx.variables);
                let result = if is_flt {
                    if negated {
                        0
                    } else {
                        1
                    }
                } else {
                    if negated {
                        1
                    } else {
                        0
                    }
                };
                arch::emit_load_num(&mut self.output, self.arch, result);
            }
            "int" | "integer" | "number" => {
                let is_num = is_number_expr(expr, &self.ctx.variables);
                let is_non = is_string_expr(expr, &self.ctx.variables)
                    || is_float_expr(expr, &self.ctx.variables)
                    || is_array_expr(expr, &self.ctx.variables)
                    || is_map_expr(expr, &self.ctx.variables)
                    || is_null_expr(expr, &self.ctx.variables);
                let result = if is_num || !is_non {
                    if negated {
                        0
                    } else {
                        1
                    }
                } else {
                    if negated {
                        1
                    } else {
                        0
                    }
                };
                arch::emit_load_num(&mut self.output, self.arch, result);
            }
            "bool" | "boolean" => {
                let is_non = is_string_expr(expr, &self.ctx.variables)
                    || is_float_expr(expr, &self.ctx.variables)
                    || is_array_expr(expr, &self.ctx.variables)
                    || is_map_expr(expr, &self.ctx.variables)
                    || is_null_expr(expr, &self.ctx.variables);
                let result = if !is_non {
                    if negated {
                        0
                    } else {
                        1
                    }
                } else {
                    if negated {
                        1
                    } else {
                        0
                    }
                };
                arch::emit_load_num(&mut self.output, self.arch, result);
            }
            _ => {
                let known_struct = match expr {
                    Expr::Identifier(id) => match self.ctx.variables.get(id) {
                        Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let is_known_target_struct = self.ctx.structs.contains_key(target)
                    || self.ctx.structs.values().any(|s| {
                        let bare = s.name.rsplit("::").next().unwrap_or(&s.name);
                        bare == target
                    });

                if let Some(sname) = known_struct {
                    let matches = sname == target || sname.ends_with(&format!("::{}", target));
                    let res = if matches {
                        if negated { 0 } else { 1 }
                    } else {
                        if negated { 1 } else { 0 }
                    };
                    arch::emit_load_num(&mut self.output, self.arch, res);
                } else if is_known_target_struct {
                    let is_def_non = is_string_expr(expr, &self.ctx.variables)
                        || is_array_expr(expr, &self.ctx.variables)
                        || is_map_expr(expr, &self.ctx.variables)
                        || is_float_expr(expr, &self.ctx.variables)
                        || is_null_expr(expr, &self.ctx.variables)
                        || is_number_expr(expr, &self.ctx.variables);
                    if is_def_non {
                        arch::emit_load_num(
                            &mut self.output,
                            self.arch,
                            if negated { 1 } else { 0 },
                        );
                    } else {
                        self.emit_struct_type_check(expr, target, negated);
                    }
                } else {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 1 } else { 0 });
                }
            }
        }
    }

    fn emit_struct_type_check(&mut self, expr: &Expr, target: &str, negated: bool) {
        self.generate_expression(expr);
        let false_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let concrete_label = self.ctx.next_label();
        let check_desc_label = self.ctx.next_label();

        let bare = target.rsplit("::").next().unwrap_or(target);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let desc_label = format!("alya_struct_desc_{}", bare);

        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    test %rax, %rax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    test $7, %rax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movq -16(%rax), %rdx\n");
                self.output.push_str("    cmp $0x5A110003, %rdx\n");
                self.output.push_str(&format!("    je {}\n", concrete_label));
                self.output.push_str("    cmp $0x5A110004, %rdx\n");
                self.output.push_str(&format!("    jne {}\n", false_label));
                // Fat pointer: load vtable at 8(%rax), then load descriptor from 0(%r11)
                self.output.push_str("    movq 8(%rax), %r11\n");
                self.output.push_str("    test %r11, %r11\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    movq (%r11), %r11\n");
                self.output.push_str(&format!("    jmp {}\n", check_desc_label));
                // Concrete struct: load descriptor from 0(%rax)
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    movq (%rax), %r11\n");
                // Check descriptor against target descriptor
                self.output.push_str(&format!("{}:\n", check_desc_label));
                self.output.push_str(&format!("    lea {}(%rip), %rdx\n", desc_label));
                self.output.push_str("    cmp %rdx, %r11\n");
                self.output.push_str(&format!("    jne {}\n", false_label));
                self.output.push_str(&format!(
                    "    movq ${}, %rax\n",
                    if negated { 0 } else { 1 }
                ));
                self.output.push_str(&format!("    jmp {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output.push_str(&format!(
                    "    movq ${}, %rax\n",
                    if negated { 1 } else { 0 }
                ));
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Architecture::X86 => {
                self.output.push_str("    test %eax, %eax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    test $3, %eax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %eax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movl -8(%eax), %edx\n");
                self.output.push_str("    cmp $0x5A110003, %edx\n");
                self.output.push_str(&format!("    je {}\n", concrete_label));
                self.output.push_str("    cmp $0x5A110004, %edx\n");
                self.output.push_str(&format!("    jne {}\n", false_label));
                // Fat pointer
                self.output.push_str("    movl 4(%eax), %ecx\n");
                self.output.push_str("    test %ecx, %ecx\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    movl (%ecx), %ecx\n");
                self.output.push_str(&format!("    jmp {}\n", check_desc_label));
                // Concrete
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    movl (%eax), %ecx\n");
                // Check desc
                self.output.push_str(&format!("{}:\n", check_desc_label));
                self.output.push_str(&format!("    cmp ${}, %ecx\n", desc_label));
                self.output.push_str(&format!("    jne {}\n", false_label));
                self.output.push_str(&format!(
                    "    movl ${}, %eax\n",
                    if negated { 0 } else { 1 }
                ));
                self.output.push_str(&format!("    jmp {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output.push_str(&format!(
                    "    movl ${}, %eax\n",
                    if negated { 1 } else { 0 }
                ));
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Architecture::ARM64 => {
                self.output.push_str("    cbz x0, ");
                self.output.push_str(&format!("{}\n", false_label));
                self.output.push_str("    tst x0, #7\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str("    cmp x0, #65536\n");
                self.output.push_str(&format!("    b.lo {}\n", false_label));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output.push_str(&format!("    cbnz x1, {}\n", false_label));
                self.output.push_str("    ldur x1, [x0, #-16]\n");
                self.output.push_str("    movz x2, #0x0003\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.eq {}\n", concrete_label));
                self.output.push_str("    movz x2, #0x0004\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                // Fat pointer
                self.output.push_str("    ldr x2, [x0, #8]\n");
                self.output.push_str(&format!("    cbz x2, {}\n", false_label));
                self.output.push_str("    ldr x2, [x2]\n");
                self.output.push_str(&format!("    b {}\n", check_desc_label));
                // Concrete
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    ldr x2, [x0]\n");
                // Check desc
                self.output.push_str(&format!("{}:\n", check_desc_label));
                crate::codegen::arch::arm64::emit_adrp_add(&mut self.output, "x3", &desc_label, self.os);
                self.output.push_str("    cmp x2, x3\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str(&format!(
                    "    mov x0, #{}\n",
                    if negated { 0 } else { 1 }
                ));
                self.output.push_str(&format!("    b {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output.push_str(&format!(
                    "    mov x0, #{}\n",
                    if negated { 1 } else { 0 }
                ));
                self.output.push_str(&format!("{}:\n", end_label));
            }
        }
    }

    fn emit_runtime_tag_check(&mut self, expr: &Expr, tag: u64, negated: bool) {
        self.generate_expression(expr);
        let false_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    test %rax, %rax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    test $7, %rax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movq -16(%rax), %rdx\n");
                self.output
                    .push_str(&format!("    cmp $0x{:X}, %rdx\n", tag));
                self.output.push_str(&format!("    jne {}\n", false_label));
                self.output.push_str(&format!(
                    "    movq ${}, %rax\n",
                    if negated { 0 } else { 1 }
                ));
                self.output.push_str(&format!("    jmp {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output.push_str(&format!(
                    "    movq ${}, %rax\n",
                    if negated { 1 } else { 0 }
                ));
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Architecture::X86 => {
                self.output.push_str("    test %eax, %eax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    test $3, %eax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %eax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movl -8(%eax), %edx\n");
                self.output
                    .push_str(&format!("    cmp $0x{:X}, %edx\n", tag as u32));
                self.output.push_str(&format!("    jne {}\n", false_label));
                self.output.push_str(&format!(
                    "    movl ${}, %eax\n",
                    if negated { 0 } else { 1 }
                ));
                self.output.push_str(&format!("    jmp {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output.push_str(&format!(
                    "    movl ${}, %eax\n",
                    if negated { 1 } else { 0 }
                ));
                self.output.push_str(&format!("{}:\n", end_label));
            }
            Architecture::ARM64 => {
                self.output.push_str("    cbz x0, ");
                self.output.push_str(&format!("{}\n", false_label));
                self.output.push_str("    tst x0, #7\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str("    cmp x0, #65536\n");
                self.output.push_str(&format!("    b.lo {}\n", false_label));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output
                    .push_str(&format!("    cbnz x1, {}\n", false_label));
                self.output.push_str("    ldur x1, [x0, #-16]\n");
                let tag_lo = tag as u32;
                self.output
                    .push_str(&format!("    movz x2, #{}\n", tag_lo & 0xFFFF));
                self.output
                    .push_str(&format!("    movk x2, #{}, lsl #16\n", tag_lo >> 16));
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output
                    .push_str(&format!("    mov x0, #{}\n", if negated { 0 } else { 1 }));
                self.output.push_str(&format!("    b {}\n", end_label));
                self.output.push_str(&format!("{}:\n", false_label));
                self.output
                    .push_str(&format!("    mov x0, #{}\n", if negated { 1 } else { 0 }));
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

    pub(crate) fn resolve_struct_field_index(&self, object: &Expr, field: &str) -> usize {
        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            _ => object,
        };

        // 1. Precise recursive type resolution
        if let Some(struct_name) = self.get_expr_struct_name(base_obj) {
            let bare = struct_name.rsplit("::").next().unwrap_or(&struct_name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if let Some(sdef) = self
                .ctx
                .structs
                .get(&struct_name)
                .or_else(|| self.ctx.structs.get(bare))
            {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    return idx;
                }
            }
        }

        // 2. Exact or suffix name matching (when variable name directly reflects struct name)
        let name_opt = match base_obj {
            Expr::Identifier(obj_name) => Some(obj_name.as_str()),
            Expr::FieldAccess {
                field: inner_field, ..
            }
            | Expr::OptionalFieldAccess {
                field: inner_field, ..
            } => Some(inner_field.as_str()),
            _ => None,
        };

        if let Some(name_str) = name_opt {
            let lower = name_str.to_lowercase();
            let norm_var = lower.replace('_', "");

            let mut matches: Vec<(i32, &String, usize)> = Vec::new();
            for (sname, sdef) in &self.ctx.structs {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    let s_lower = sname.to_lowercase();
                    let s_bare = s_lower.rsplit("::").next().unwrap_or(&s_lower);
                    let s_bare = s_bare.rsplit("__").next().unwrap_or(s_bare);
                    let norm_struct = s_bare.replace('_', "");

                    let score = if norm_struct == norm_var {
                        3
                    } else if norm_struct.ends_with(&norm_var) {
                        2
                    } else if norm_var.ends_with(&norm_struct) {
                        1
                    } else {
                        0
                    };

                    if score > 0 {
                        matches.push((score, sname, idx));
                    }
                }
            }

            if !matches.is_empty() {
                matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
                return matches[0].2;
            }
        }

        // 3. Fallback: match any struct containing this field (sorted alphabetically for determinism)
        let mut candidates: Vec<(&String, usize)> = Vec::new();
        for (sname, sdef) in &self.ctx.structs {
            if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                candidates.push((sname, idx));
            }
        }

        if !candidates.is_empty() {
            candidates.sort_by(|a, b| a.0.cmp(b.0));
            let first_idx = candidates[0].1;
            if candidates.iter().any(|c| c.1 != first_idx) {
                let mut distinct: Vec<(&str, usize)> = Vec::new();
                for (sname, idx) in &candidates {
                    let bare = sname.rsplit("::").next().unwrap_or(sname);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if !distinct.iter().any(|(b, _)| *b == bare) {
                        distinct.push((bare, *idx));
                    }
                }
                let details = distinct
                    .iter()
                    .map(|(s, i)| format!("{}.{} (index {})", s, field, i))
                    .collect::<Vec<_>>()
                    .join(", ");
                let obj_desc = match base_obj {
                    Expr::Identifier(id) => format!("'{}'", id),
                    _ => "object".to_string(),
                };
                let in_fn = if self.ctx.current_fn_name.is_empty() {
                    String::new()
                } else {
                    format!(" in function '{}'", self.ctx.current_fn_name)
                };
                eprintln!(
                    "warning: ambiguous field access '.{}' on untyped {}{}. Conflicting layouts found: {}. Please specify a type annotation.",
                    field, obj_desc, in_fn, details
                );
            }
            return candidates[0].1;
        }

        0
    }

    pub(crate) fn is_struct_field_weak(&self, object: &Expr, field: &str) -> bool {
        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            _ => object,
        };

        if let Some(struct_name) = self.get_expr_struct_name(base_obj) {
            let bare = struct_name.rsplit("::").next().unwrap_or(&struct_name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if let Some(sdef) = self
                .ctx
                .structs
                .get(&struct_name)
                .or_else(|| self.ctx.structs.get(bare))
            {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    if let Some(Some(ft)) = sdef.field_types.get(idx) {
                        return ft.starts_with("weak ") || ft == "weak";
                    }
                }
            }
        }

        let name_opt = match base_obj {
            Expr::Identifier(obj_name) => Some(obj_name.as_str()),
            Expr::FieldAccess {
                field: inner_field, ..
            }
            | Expr::OptionalFieldAccess {
                field: inner_field, ..
            } => Some(inner_field.as_str()),
            _ => None,
        };

        if let Some(name_str) = name_opt {
            let lower = name_str.to_lowercase();
            let norm_var = lower.replace('_', "");
            for (sname, sdef) in &self.ctx.structs {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    let s_lower = sname.to_lowercase();
                    let s_bare = s_lower.rsplit("::").next().unwrap_or(&s_lower);
                    let s_bare = s_bare.rsplit("__").next().unwrap_or(s_bare);
                    let norm_struct = s_bare.replace('_', "");
                    if norm_struct == norm_var
                        || norm_struct.ends_with(&norm_var)
                        || norm_var.ends_with(&norm_struct)
                    {
                        if let Some(Some(ft)) = sdef.field_types.get(idx) {
                            return ft.starts_with("weak ") || ft == "weak";
                        }
                    }
                }
            }
        }

        for (_sname, sdef) in &self.ctx.structs {
            if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                if let Some(Some(ft)) = sdef.field_types.get(idx) {
                    if ft.starts_with("weak ") || ft == "weak" {
                        return true;
                    }
                }
            }
        }
        false
    }
}
