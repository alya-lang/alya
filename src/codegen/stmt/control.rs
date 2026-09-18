use super::CodeGen;
use crate::ast::{BinaryOp, Expr, Stmt};
use crate::codegen::analysis::{
    is_float_array, is_float_expr, is_map_expr, is_string_array, is_string_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

impl CodeGen {
    pub(crate) fn generate_condition_jump_if_false(
        &mut self,
        condition: &Expr,
        target_label: &str,
    ) {
        if let Expr::Binary { left, op, right } = condition {
            if *op == BinaryOp::And {
                self.generate_condition_jump_if_false(left, target_label);
                self.generate_condition_jump_if_false(right, target_label);
                return;
            }

            if *op == BinaryOp::Or {
                let pass_label = self.ctx.next_label();
                self.generate_condition_jump_if_true(left, &pass_label);
                self.generate_condition_jump_if_false(right, target_label);
                self.output.push_str(&format!("{}:\n", pass_label));
                return;
            }

            let is_cmp = matches!(
                op,
                BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
            );
            if is_cmp
                && !is_string_expr(left, &self.ctx.variables)
                && !is_string_expr(right, &self.ctx.variables)
            {
                let left_is_flt = is_float_expr(left, &self.ctx.variables);
                let right_is_flt = is_float_expr(right, &self.ctx.variables);
                if left_is_flt || right_is_flt {
                    self.generate_expression(left);
                    if !left_is_flt {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    if let Expr::Float(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Expr::Number(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Some(&VarType::Float(offset)) = match &**right {
                        Expr::Identifier(var_name) => self.ctx.variables.get(var_name),
                        _ => None,
                    } {
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, true);
                        arch::emit_float_cmp_reg(&mut self.output, self.arch);
                        arch::emit_float_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            true,
                            target_label,
                        );
                        return;
                    } else {
                        arch::emit_push_temp(&mut self.output, self.arch);
                        self.generate_expression(right);
                        if !right_is_flt {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                        arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                    }
                    arch::emit_jump_if_zero(&mut self.output, self.arch, target_label);
                    return;
                }

                if let Expr::Number(n) = &**right {
                    self.generate_expression(left);
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    arch::emit_cond_jump(&mut self.output, self.arch, *op, true, target_label);
                    return;
                }
                if let Expr::Identifier(var_name) = &**right {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(left);
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        arch::emit_cond_jump(&mut self.output, self.arch, *op, true, target_label);
                        return;
                    }
                }
                if let Expr::Number(n) = &**left {
                    self.generate_expression(right);
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let swapped_op = match op {
                        BinaryOp::Less => BinaryOp::Greater,
                        BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                        BinaryOp::Greater => BinaryOp::Less,
                        BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                        other => *other,
                    };
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        swapped_op,
                        true,
                        target_label,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**left {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(right);
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let swapped_op = match op {
                            BinaryOp::Less => BinaryOp::Greater,
                            BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                            BinaryOp::Greater => BinaryOp::Less,
                            BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                            other => *other,
                        };
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            swapped_op,
                            true,
                            target_label,
                        );
                        return;
                    }
                }
            }
        }

        self.generate_expression(condition);
        arch::emit_jump_if_zero(&mut self.output, self.arch, target_label);
    }

    pub(crate) fn generate_condition_jump_if_true(&mut self, condition: &Expr, target_label: &str) {
        if let Expr::Binary { left, op, right } = condition {
            if *op == BinaryOp::Or {
                self.generate_condition_jump_if_true(left, target_label);
                self.generate_condition_jump_if_true(right, target_label);
                return;
            }

            if *op == BinaryOp::And {
                let fail_label = self.ctx.next_label();
                self.generate_condition_jump_if_false(left, &fail_label);
                self.generate_condition_jump_if_true(right, target_label);
                self.output.push_str(&format!("{}:\n", fail_label));
                return;
            }

            let is_cmp = matches!(
                op,
                BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
            );
            if is_cmp
                && !is_string_expr(left, &self.ctx.variables)
                && !is_string_expr(right, &self.ctx.variables)
            {
                let left_is_flt = is_float_expr(left, &self.ctx.variables);
                let right_is_flt = is_float_expr(right, &self.ctx.variables);
                if left_is_flt || right_is_flt {
                    self.generate_expression(left);
                    if !left_is_flt {
                        arch::emit_int_to_float(&mut self.output, self.arch);
                    }
                    if let Expr::Float(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Expr::Number(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Some(&VarType::Float(offset)) = match &**right {
                        Expr::Identifier(var_name) => self.ctx.variables.get(var_name),
                        _ => None,
                    } {
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, true);
                        arch::emit_float_cmp_reg(&mut self.output, self.arch);
                        arch::emit_float_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            false,
                            target_label,
                        );
                        return;
                    } else {
                        arch::emit_push_temp(&mut self.output, self.arch);
                        self.generate_expression(right);
                        if !right_is_flt {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                        arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                    }
                    arch::emit_jump_if_not_zero(&mut self.output, self.arch, target_label);
                    return;
                }

                if let Expr::Number(n) = &**right {
                    self.generate_expression(left);
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    arch::emit_cond_jump(&mut self.output, self.arch, *op, false, target_label);
                    return;
                }
                if let Expr::Identifier(var_name) = &**right {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(left);
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        arch::emit_cond_jump(&mut self.output, self.arch, *op, false, target_label);
                        return;
                    }
                }
                if let Expr::Number(n) = &**left {
                    self.generate_expression(right);
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let swapped_op = match op {
                        BinaryOp::Less => BinaryOp::Greater,
                        BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                        BinaryOp::Greater => BinaryOp::Less,
                        BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                        other => *other,
                    };
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        swapped_op,
                        false,
                        target_label,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**left {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(right);
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let swapped_op = match op {
                            BinaryOp::Less => BinaryOp::Greater,
                            BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                            BinaryOp::Greater => BinaryOp::Less,
                            BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                            other => *other,
                        };
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            swapped_op,
                            false,
                            target_label,
                        );
                        return;
                    }
                }
            }
        }

        self.generate_expression(condition);
        arch::emit_jump_if_not_zero(&mut self.output, self.arch, target_label);
    }

    pub(super) fn generate_if(
        &mut self,
        condition: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
    ) {
        let else_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();

        let target_label = if else_block.is_some() {
            &else_label
        } else {
            &end_label
        };
        self.generate_condition_jump_if_false(condition, target_label);

        let initial_stack_offset = self.ctx.stack_offset;
        let initial_variables = self.ctx.variables.clone();

        for s in then_block {
            self.generate_statement(s);
        }

        let then_delta = self.ctx.stack_offset - initial_stack_offset;
        if then_delta > 0 {
            arch::emit_stack_restore(&mut self.output, self.arch, then_delta);
        }

        if let Some(else_stmts) = else_block {
            arch::emit_jump(&mut self.output, self.arch, &end_label);
            self.output.push_str(&format!("{}:\n", else_label));

            self.ctx.stack_offset = initial_stack_offset;
            self.ctx.variables = initial_variables.clone();

            for s in else_stmts {
                self.generate_statement(s);
            }

            let else_delta = self.ctx.stack_offset - initial_stack_offset;
            if else_delta > 0 {
                arch::emit_stack_restore(&mut self.output, self.arch, else_delta);
            }
        }

        self.output.push_str(&format!("{}:\n", end_label));
        self.ctx.stack_offset = initial_stack_offset;
        self.ctx.variables = initial_variables;
    }

    pub(super) fn generate_while(&mut self, condition: &Expr, body: &[Stmt]) {
        let start_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let loop_body_stack_offset = self.ctx.stack_offset;
        let loop_body_variables = self.ctx.variables.clone();

        self.ctx.push_loop(
            start_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        self.generate_condition_jump_if_false(condition, &end_label);

        for s in body {
            self.generate_statement(s);
        }

        let body_delta = self.ctx.stack_offset - loop_body_stack_offset;
        if body_delta > 0 {
            arch::emit_stack_restore(&mut self.output, self.arch, body_delta);
        }
        self.ctx.stack_offset = loop_body_stack_offset;
        self.ctx.variables = loop_body_variables;

        arch::emit_jump(&mut self.output, self.arch, &start_label);
        self.output.push_str(&format!("{}:\n", end_label));

        self.ctx.pop_loop();
    }

    pub(super) fn generate_repeat(&mut self, body: &[Stmt]) {
        let start_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let loop_body_stack_offset = self.ctx.stack_offset;
        let loop_body_variables = self.ctx.variables.clone();

        self.ctx.push_loop(
            start_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));

        for s in body {
            self.generate_statement(s);
        }

        let body_delta = self.ctx.stack_offset - loop_body_stack_offset;
        if body_delta > 0 {
            arch::emit_stack_restore(&mut self.output, self.arch, body_delta);
        }
        self.ctx.stack_offset = loop_body_stack_offset;
        self.ctx.variables = loop_body_variables;

        arch::emit_jump(&mut self.output, self.arch, &start_label);
        self.output.push_str(&format!("{}:\n", end_label));

        self.ctx.pop_loop();
    }

    pub(super) fn generate_for(&mut self, var: &str, start: &Expr, end: &Expr, body: &[Stmt]) {
        let var = var.to_string();
        self.generate_expression(start);

        let var_offset = match self.ctx.variables.get(&var) {
            Some(VarType::Number(offset)) => {
                let off = *offset;
                arch::emit_store_var(&mut self.output, self.arch, off, self.ctx.stack_offset);
                off
            }
            _ => {
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                self.ctx
                    .variables
                    .insert(var.clone(), VarType::Number(self.ctx.stack_offset));
                self.ctx.stack_offset
            }
        };

        let start_label = self.ctx.next_label();
        let step_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let loop_body_stack_offset = self.ctx.stack_offset;
        let loop_body_variables = self.ctx.variables.clone();

        self.ctx.push_loop(
            step_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        arch::emit_load_var(
            &mut self.output,
            self.arch,
            var_offset,
            self.ctx.stack_offset,
        );
        arch::emit_push_temp(&mut self.output, self.arch);

        self.generate_expression(end);
        arch::emit_compare_and_jump_if_greater(&mut self.output, self.arch, &end_label);

        for s in body {
            self.generate_statement(s);
        }

        let body_delta = self.ctx.stack_offset - loop_body_stack_offset;
        if body_delta > 0 {
            arch::emit_stack_restore(&mut self.output, self.arch, body_delta);
        }
        self.ctx.stack_offset = loop_body_stack_offset;
        self.ctx.variables = loop_body_variables;

        self.output.push_str(&format!("{}:\n", step_label));
        arch::emit_increment_var(
            &mut self.output,
            self.arch,
            var_offset,
            self.ctx.stack_offset,
            &start_label,
        );
        self.output.push_str(&format!("{}:\n", end_label));

        self.ctx.pop_loop();
    }

    pub(super) fn generate_for_each(
        &mut self,
        var: &str,
        value_var: Option<&str>,
        iterable: &Expr,
        body: &[Stmt],
    ) {
        let var = var.to_string();
        self.generate_expression(iterable);
        arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
        let arr_offset = self.ctx.stack_offset;

        arch::emit_load_num(&mut self.output, self.arch, 0);
        arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
        let idx_offset = self.ctx.stack_offset;

        let is_str = is_string_array(iterable, &self.ctx.variables);
        let is_flt = is_float_array(iterable, &self.ctx.variables);

        let inferred_struct_type = match iterable {
            Expr::Identifier(arr_name) => {
                match self
                    .ctx
                    .variables
                    .get(&format!("arr_struct_type:{}", arr_name))
                {
                    Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                    _ => None,
                }
            }
            Expr::Array(elems) => elems.first().and_then(|e| match e {
                Expr::StructInit { name, .. } => Some(name.clone()),
                Expr::Call { name, .. } if self.ctx.structs.contains_key(name) => {
                    Some(name.clone())
                }
                Expr::Identifier(id) => match self.ctx.variables.get(id) {
                    Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                    _ => None,
                },
                _ => None,
            }),
            _ => None,
        };

        let is_channel = inferred_struct_type.as_deref() == Some("Channel")
            || self.get_expr_struct_name(iterable).as_deref() == Some("Channel");

        if is_channel {
            self.generate_expression(iterable);
            arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
            let ch_offset = self.ctx.stack_offset;

            arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
            let var_offset = self.ctx.stack_offset;
            self.ctx
                .variables
                .insert(var.to_string(), VarType::Number(var_offset));

            let start_label = self.ctx.next_label();
            let step_label = self.ctx.next_label();
            let end_label = self.ctx.next_label();
            let loop_body_stack_offset = self.ctx.stack_offset;
            let loop_body_variables = self.ctx.variables.clone();

            self.ctx.push_loop(
                step_label.clone(),
                end_label.clone(),
                loop_body_stack_offset,
            );

            self.output.push_str(&format!("{}:\n", start_label));

            let initial_stack_offset = self.ctx.stack_offset;
            match self.arch {
                Architecture::X86 => {
                    arch::emit_load_num(&mut self.output, self.arch, 5000);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    arch::emit_load_var(&mut self.output, self.arch, ch_offset, initial_stack_offset + 4);
                    arch::emit_push_temp(&mut self.output, self.arch);
                }
                _ => {
                    arch::emit_load_var(&mut self.output, self.arch, ch_offset, initial_stack_offset);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    arch::emit_load_num(&mut self.output, self.arch, 5000);
                    arch::emit_push_temp(&mut self.output, self.arch);
                }
            }

            let recv_fn = self
                .ctx
                .functions
                .iter()
                .find(|f| f.ends_with("Channel__recv") || f.ends_with("channel_recv"))
                .map(|s| s.as_str())
                .unwrap_or("channel_recv");
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                recv_fn,
                2,
                initial_stack_offset,
                self.os,
            );
            self.ctx.stack_offset = initial_stack_offset;

            arch::emit_cmp_imm(&mut self.output, self.arch, 0);
            arch::emit_cond_jump(
                &mut self.output,
                self.arch,
                BinaryOp::Equal,
                false,
                &end_label,
            );

            arch::emit_store_var(&mut self.output, self.arch, var_offset, self.ctx.stack_offset);

            for s in body {
                self.generate_statement(s);
            }

            let body_delta = self.ctx.stack_offset - loop_body_stack_offset;
            if body_delta > 0 {
                arch::emit_stack_restore(&mut self.output, self.arch, body_delta);
            }
            self.ctx.stack_offset = loop_body_stack_offset;
            self.ctx.variables = loop_body_variables;

            self.output.push_str(&format!("{}:\n", step_label));
            arch::emit_jump(&mut self.output, self.arch, &start_label);
            self.output.push_str(&format!("{}:\n", end_label));

            self.ctx.pop_loop();
            return;
        }

        let is_map = is_map_expr(iterable, &self.ctx.variables);
        let is_map_str_val = is_map
            && match iterable {
                Expr::Map(entries) => entries
                    .iter()
                    .any(|(_, v)| is_string_expr(v, &self.ctx.variables)),
                Expr::Identifier(name) => self
                    .ctx
                    .variables
                    .keys()
                    .any(|k| k.starts_with(&format!("map_str:{}.", name))),
                _ => false,
            };

        let var_type_for_primary = if value_var.is_some() {
            if is_map {
                VarType::StringOffset(0)
            } else {
                VarType::Number(0)
            }
        } else if is_map {
            VarType::StringOffset(0)
        } else if let Some(sname) = &inferred_struct_type {
            VarType::Struct {
                struct_name: sname.clone(),
                offset: 0,
            }
        } else if is_str {
            VarType::StringOffset(0)
        } else if is_flt {
            VarType::Float(0)
        } else {
            VarType::Number(0)
        };

        let set_var_offset = |vt: &VarType, off: i32| -> VarType {
            match vt {
                VarType::Number(_) => VarType::Number(off),
                VarType::Float(_) => VarType::Float(off),
                VarType::StringOffset(_) => VarType::StringOffset(off),
                VarType::Array(_) => VarType::Array(off),
                VarType::Map(_) => VarType::Map(off),
                VarType::Null(_) => VarType::Null(off),
                VarType::Struct { struct_name, .. } => VarType::Struct {
                    struct_name: struct_name.clone(),
                    offset: off,
                },
                VarType::StringLabel(s) => VarType::StringLabel(s.clone()),
            }
        };

        let var_offset = match self.ctx.variables.get(&var) {
            Some(
                VarType::Number(offset)
                | VarType::Float(offset)
                | VarType::StringOffset(offset)
                | VarType::Struct { offset, .. },
            ) => {
                let off = *offset;
                let vt = set_var_offset(&var_type_for_primary, off);
                self.ctx.variables.insert(var.clone(), vt);
                off
            }
            _ => {
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                let off = self.ctx.stack_offset;
                let vt = set_var_offset(&var_type_for_primary, off);
                self.ctx.variables.insert(var.clone(), vt);
                off
            }
        };

        let val_offset = if let Some(v2_name) = value_var {
            let v2_str = v2_name.to_string();
            let v2_type_raw = if let Some(sname) = &inferred_struct_type {
                VarType::Struct {
                    struct_name: sname.clone(),
                    offset: 0,
                }
            } else if is_str || is_map_str_val {
                VarType::StringOffset(0)
            } else if is_flt {
                VarType::Float(0)
            } else {
                VarType::Number(0)
            };

            let off2 = match self.ctx.variables.get(&v2_str) {
                Some(
                    VarType::Number(offset)
                    | VarType::Float(offset)
                    | VarType::StringOffset(offset)
                    | VarType::Struct { offset, .. },
                ) => {
                    let off = *offset;
                    let vt = set_var_offset(&v2_type_raw, off);
                    self.ctx.variables.insert(v2_str.clone(), vt);
                    off
                }
                _ => {
                    arch::emit_allocate_var(
                        &mut self.output,
                        self.arch,
                        &mut self.ctx.stack_offset,
                    );
                    let off = self.ctx.stack_offset;
                    let vt = set_var_offset(&v2_type_raw, off);
                    self.ctx.variables.insert(v2_str.clone(), vt);
                    off
                }
            };
            Some(off2)
        } else {
            None
        };

        let target_struct_var = value_var.unwrap_or(&var);
        if let Some(sname) = &inferred_struct_type {
            let bare = sname.rsplit("::").next().unwrap_or(sname);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if let Some(sdef) = self
                .ctx
                .structs
                .get(sname)
                .or_else(|| self.ctx.structs.get(bare))
                .cloned()
            {
                for fname in &sdef.fields {
                    let field_key = format!("{}.{}", target_struct_var, fname);
                    if self
                        .ctx
                        .variables
                        .contains_key(&format!("struct_field_str:{}.{}", sname, fname))
                        || self
                            .ctx
                            .variables
                            .contains_key(&format!("struct_field_str:{}", fname))
                    {
                        self.ctx
                            .variables
                            .insert(field_key, VarType::StringOffset(0));
                    } else if self
                        .ctx
                        .variables
                        .contains_key(&format!("struct_field_flt:{}.{}", sname, fname))
                        || self
                            .ctx
                            .variables
                            .contains_key(&format!("struct_field_flt:{}", fname))
                    {
                        self.ctx.variables.insert(field_key, VarType::Float(0));
                    } else {
                        self.ctx.variables.insert(field_key, VarType::Number(0));
                    }
                }
            }
        }

        let start_label = self.ctx.next_label();
        let step_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let map_label = self.ctx.next_label();
        let done_label = self.ctx.next_label();
        let loop_body_stack_offset = self.ctx.stack_offset;
        let loop_body_variables = self.ctx.variables.clone();

        self.ctx.push_loop(
            step_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        arch::emit_for_each_load_element(
            &mut self.output,
            self.arch,
            arr_offset,
            idx_offset,
            var_offset,
            val_offset,
            &end_label,
            &map_label,
            &done_label,
        );

        for s in body {
            self.generate_statement(s);
        }

        let body_delta = self.ctx.stack_offset - loop_body_stack_offset;
        if body_delta > 0 {
            arch::emit_stack_restore(&mut self.output, self.arch, body_delta);
        }
        self.ctx.stack_offset = loop_body_stack_offset;
        self.ctx.variables = loop_body_variables;

        self.output.push_str(&format!("{}:\n", step_label));
        arch::emit_increment_var(
            &mut self.output,
            self.arch,
            idx_offset,
            self.ctx.stack_offset,
            &start_label,
        );
        self.output.push_str(&format!("{}:\n", end_label));

        self.ctx.pop_loop();
    }

    pub(super) fn generate_throw(&mut self, opt_expr: Option<&Expr>) {
        if let Some(expr) = opt_expr {
            if is_string_expr(expr, &self.ctx.variables) {
                self.generate_expression(expr);
                arch::emit_push_temp(&mut self.output, self.arch);
            } else {
                self.generate_expression(expr);
                arch::emit_push_temp(&mut self.output, self.arch);
                arch::emit_function_call(
                    &mut self.output,
                    self.arch,
                    "str",
                    1,
                    self.ctx.stack_offset,
                    self.os,
                );
                arch::emit_push_temp(&mut self.output, self.arch);
            }
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "throw",
                1,
                self.ctx.stack_offset,
                self.os,
            );
        } else {
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "rethrow",
                0,
                self.ctx.stack_offset,
                self.os,
            );
        }
    }

    pub(super) fn generate_try_catch(
        &mut self,
        try_block: &[Stmt],
        catch_var: Option<&str>,
        catch_block: &[Stmt],
        finally_block: Option<&[Stmt]>,
    ) {
        let catch_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let saved_stack_offset = self.ctx.stack_offset;
        let saved_variables = self.ctx.variables.clone();

        if let Some(finally_stmts) = finally_block {
            let finally_normal_label = self.ctx.next_label();
            let finally_rethrow_label = self.ctx.next_label();

            if !catch_block.is_empty() || catch_var.is_some() {
                // Try block with catch
                arch::emit_try_begin(&mut self.output, self.arch, &catch_label, self.os);
                for s in try_block {
                    self.generate_statement(s);
                }
                let try_delta = self.ctx.stack_offset - saved_stack_offset;
                arch::emit_try_end(
                    &mut self.output,
                    self.arch,
                    &finally_normal_label,
                    try_delta,
                    self.os,
                );

                // Catch block
                self.ctx.stack_offset = saved_stack_offset;
                self.ctx.variables = saved_variables.clone();
                arch::emit_catch_begin(&mut self.output, self.arch, &catch_label);

                // Temporary try handler so that errors inside catch run finally and rethrow
                arch::emit_try_begin(&mut self.output, self.arch, &finally_rethrow_label, self.os);

                if let Some(name) = catch_var {
                    let name = name.to_string();
                    arch::emit_catch_load_err(&mut self.output, self.arch, self.os);
                    arch::emit_allocate_var(
                        &mut self.output,
                        self.arch,
                        &mut self.ctx.stack_offset,
                    );
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
                }

                for s in catch_block {
                    self.generate_statement(s);
                }
                let catch_delta = self.ctx.stack_offset - saved_stack_offset;
                arch::emit_try_end(
                    &mut self.output,
                    self.arch,
                    &finally_normal_label,
                    catch_delta,
                    self.os,
                );
            } else {
                // Try block WITHOUT catch (only finally)
                arch::emit_try_begin(&mut self.output, self.arch, &finally_rethrow_label, self.os);
                for s in try_block {
                    self.generate_statement(s);
                }
                let try_delta = self.ctx.stack_offset - saved_stack_offset;
                arch::emit_try_end(
                    &mut self.output,
                    self.arch,
                    &finally_normal_label,
                    try_delta,
                    self.os,
                );
            }

            // Normal path to finally
            self.output
                .push_str(&format!("{}:\n", finally_normal_label));
            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables.clone();
            for s in finally_stmts {
                self.generate_statement(s);
            }
            let finally_delta = self.ctx.stack_offset - saved_stack_offset;
            arch::emit_catch_end(&mut self.output, self.arch, finally_delta);
            arch::emit_jump(&mut self.output, self.arch, &end_label);

            // Error path to finally (runs finally and rethrows)
            self.output
                .push_str(&format!("{}:\n", finally_rethrow_label));
            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables.clone();
            for s in finally_stmts {
                self.generate_statement(s);
            }
            let finally_rethrow_delta = self.ctx.stack_offset - saved_stack_offset;
            arch::emit_catch_end(&mut self.output, self.arch, finally_rethrow_delta);
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "rethrow",
                0,
                self.ctx.stack_offset,
                self.os,
            );

            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables;
            self.output.push_str(&format!("{}:\n", end_label));
        } else {
            // Existing try-catch without finally
            arch::emit_try_begin(&mut self.output, self.arch, &catch_label, self.os);

            for s in try_block {
                self.generate_statement(s);
            }

            let try_delta = self.ctx.stack_offset - saved_stack_offset;
            arch::emit_try_end(&mut self.output, self.arch, &end_label, try_delta, self.os);

            // At catch entry, runtime SP has been restored to saved_stack_offset.
            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables.clone();
            arch::emit_catch_begin(&mut self.output, self.arch, &catch_label);

            if let Some(name) = catch_var {
                let name = name.to_string();
                arch::emit_catch_load_err(&mut self.output, self.arch, self.os);
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
            }

            for s in catch_block {
                self.generate_statement(s);
            }

            let catch_delta = self.ctx.stack_offset - saved_stack_offset;
            arch::emit_catch_end(&mut self.output, self.arch, catch_delta);

            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables;
            self.output.push_str(&format!("{}:\n", end_label));
        }
    }
}
