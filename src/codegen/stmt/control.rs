use super::CodeGen;
use crate::ast::{BinaryOp, Expr, Stmt};
use crate::codegen::analysis::{
    is_array_expr, is_definitely_not_numeric, is_float_array, is_float_expr, is_map_expr,
    is_string_array, is_string_expr, is_tag_carrying_read, is_unsigned_expr,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::kinds::KIND_FLOAT;
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
            let has_overloaded_op = if is_cmp {
                if let Some(sname) = self.get_expr_struct_name(left) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let op_str = match op {
                        BinaryOp::Equal => Some("=="),
                        BinaryOp::NotEqual => Some("!="),
                        BinaryOp::Less => Some("<"),
                        BinaryOp::Greater => Some(">"),
                        BinaryOp::LessEqual => Some("<="),
                        BinaryOp::GreaterEqual => Some(">="),
                        _ => None,
                    };
                    if let Some(op_sym) = op_str {
                        let cand1 = format!("{}__{}{}", sname, "operator", op_sym);
                        let cand2 = format!("{}__{}{}", bare_sname, "operator", op_sym);
                        self.ctx.functions.contains(&cand1)
                            || self.ctx.functions.contains(&cand2)
                            || self
                                .ctx
                                .functions
                                .iter()
                                .any(|f| f.ends_with(&format!("__{}{}", "operator", op_sym)))
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            if is_cmp
                && !has_overloaded_op
                && !is_string_expr(left, &self.ctx.variables)
                && !is_string_expr(right, &self.ctx.variables)
            {
                let left_is_flt = is_float_expr(left, &self.ctx.variables);
                let right_is_flt = is_float_expr(right, &self.ctx.variables);
                if left_is_flt || right_is_flt {
                    self.generate_expression(left);
                    if !left_is_flt && !is_definitely_not_numeric(left, &self.ctx.variables) {
                        // Index carries kind tag alongside the value
                        // (x64: %edx, arm64: w1): skip int->float
                        // when the value is already a float (map routing
                        // or Phase 1 array slot kinds).
                        if matches!(
                            &**left,
                            Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                        ) && is_tag_carrying_read(left, &self.ctx.variables)
                        {
                            let l_skip = self.ctx.next_label();
                            if matches!(self.arch, Architecture::X64) {
                                self.output.push_str("    cmpl $2, %edx\n");
                                self.output.push_str(&format!("    je {}\n", l_skip));
                            } else {
                                self.output.push_str("    cmp w1, #2\n");
                                self.output.push_str(&format!("    b.eq {}\n", l_skip));
                            }
                            arch::emit_int_to_float(&mut self.output, self.arch);
                            self.output.push_str(&format!("{}:\n", l_skip));
                        } else {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                    }
                    if let Expr::Float(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Expr::Number(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n as f64);
                    } else if let Some(&VarType::Float(offset)) = match &**right {
                        Expr::Identifier(var_name) => self.ctx.variables.get(var_name),
                        _ => None,
                    } {
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, true);
                        arch::emit_float_cmp_reg(&mut self.output, self.arch);
                        let skip_label = self.ctx.next_label();
                        arch::emit_float_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            true,
                            target_label,
                            &skip_label,
                        );
                        return;
                    } else {
                        arch::emit_push_temp(&mut self.output, self.arch);
                        self.generate_expression(right);
                        if !right_is_flt && !is_definitely_not_numeric(right, &self.ctx.variables) {
                            // Index carries kind tag (x64: %edx, arm64: w1;
                            // map routing or Phase 1 array slot kinds).
                            if matches!(
                                &**right,
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            ) && is_tag_carrying_read(right, &self.ctx.variables)
                            {
                                let l_skip = self.ctx.next_label();
                                if matches!(self.arch, Architecture::X64) {
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    je {}\n", l_skip));
                                } else {
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    b.eq {}\n", l_skip));
                                }
                                arch::emit_int_to_float(&mut self.output, self.arch);
                                self.output.push_str(&format!("{}:\n", l_skip));
                            } else {
                                arch::emit_int_to_float(&mut self.output, self.arch);
                            }
                        }
                        arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                    }
                    arch::emit_jump_if_zero(&mut self.output, self.arch, target_label);
                    return;
                }

                if let Expr::Number(n) = &**right {
                    self.generate_expression(left);
                    // Strict dynamic check (#39 Phase 2b): float tag on a
                    // tag-carrying read is the runtime half of the mixed
                    // comparison error.
                    if is_tag_carrying_read(left, &self.ctx.variables) {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                        || is_unsigned_expr(right, &self.ctx.variables);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        *op,
                        true,
                        target_label,
                        unsigned,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**right {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(left);
                        // Strict dynamic check (#39 Phase 2b): float tag on a
                        // tag-carrying read is the runtime half of the mixed
                        // comparison error.
                        if is_tag_carrying_read(left, &self.ctx.variables) {
                            arch::emit_mixed_float_check(&mut self.output, self.arch);
                        }
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                            || is_unsigned_expr(right, &self.ctx.variables);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            true,
                            target_label,
                            unsigned,
                        );
                        return;
                    }
                }
                if let Expr::Number(n) = &**left {
                    self.generate_expression(right);
                    // Strict dynamic check (#39 Phase 2b): float tag on a
                    // tag-carrying read is the runtime half of the mixed
                    // comparison error.
                    if is_tag_carrying_read(right, &self.ctx.variables) {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let swapped_op = match op {
                        BinaryOp::Less => BinaryOp::Greater,
                        BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                        BinaryOp::Greater => BinaryOp::Less,
                        BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                        other => *other,
                    };
                    let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                        || is_unsigned_expr(right, &self.ctx.variables);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        swapped_op,
                        true,
                        target_label,
                        unsigned,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**left {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(right);
                        // Strict dynamic check (#39 Phase 2b): float tag on a
                        // tag-carrying read is the runtime half of the mixed
                        // comparison error.
                        if is_tag_carrying_read(right, &self.ctx.variables) {
                            arch::emit_mixed_float_check(&mut self.output, self.arch);
                        }
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let swapped_op = match op {
                            BinaryOp::Less => BinaryOp::Greater,
                            BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                            BinaryOp::Greater => BinaryOp::Less,
                            BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                            other => *other,
                        };
                        let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                            || is_unsigned_expr(right, &self.ctx.variables);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            swapped_op,
                            true,
                            target_label,
                            unsigned,
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
            let has_overloaded_op = if is_cmp {
                if let Some(sname) = self.get_expr_struct_name(left) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let op_str = match op {
                        BinaryOp::Equal => Some("=="),
                        BinaryOp::NotEqual => Some("!="),
                        BinaryOp::Less => Some("<"),
                        BinaryOp::Greater => Some(">"),
                        BinaryOp::LessEqual => Some("<="),
                        BinaryOp::GreaterEqual => Some(">="),
                        _ => None,
                    };
                    if let Some(op_sym) = op_str {
                        let cand1 = format!("{}__{}{}", sname, "operator", op_sym);
                        let cand2 = format!("{}__{}{}", bare_sname, "operator", op_sym);
                        self.ctx.functions.contains(&cand1)
                            || self.ctx.functions.contains(&cand2)
                            || self
                                .ctx
                                .functions
                                .iter()
                                .any(|f| f.ends_with(&format!("__{}{}", "operator", op_sym)))
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            if is_cmp
                && !has_overloaded_op
                && !is_string_expr(left, &self.ctx.variables)
                && !is_string_expr(right, &self.ctx.variables)
            {
                let left_is_flt = is_float_expr(left, &self.ctx.variables);
                let right_is_flt = is_float_expr(right, &self.ctx.variables);
                if left_is_flt || right_is_flt {
                    self.generate_expression(left);
                    if !left_is_flt && !is_definitely_not_numeric(left, &self.ctx.variables) {
                        // Index carries kind tag alongside the value
                        // (x64: %edx, arm64: w1): skip int->float
                        // when the value is already a float (map routing
                        // or Phase 1 array slot kinds).
                        if matches!(
                            &**left,
                            Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                        ) && is_tag_carrying_read(left, &self.ctx.variables)
                        {
                            let l_skip = self.ctx.next_label();
                            if matches!(self.arch, Architecture::X64) {
                                self.output.push_str("    cmpl $2, %edx\n");
                                self.output.push_str(&format!("    je {}\n", l_skip));
                            } else {
                                self.output.push_str("    cmp w1, #2\n");
                                self.output.push_str(&format!("    b.eq {}\n", l_skip));
                            }
                            arch::emit_int_to_float(&mut self.output, self.arch);
                            self.output.push_str(&format!("{}:\n", l_skip));
                        } else {
                            arch::emit_int_to_float(&mut self.output, self.arch);
                        }
                    }
                    if let Expr::Float(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                    } else if let Expr::Number(n) = &**right {
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n as f64);
                    } else if let Some(&VarType::Float(offset)) = match &**right {
                        Expr::Identifier(var_name) => self.ctx.variables.get(var_name),
                        _ => None,
                    } {
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, true);
                        arch::emit_float_cmp_reg(&mut self.output, self.arch);
                        let skip_label = self.ctx.next_label();
                        arch::emit_float_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            false,
                            target_label,
                            &skip_label,
                        );
                        return;
                    } else {
                        arch::emit_push_temp(&mut self.output, self.arch);
                        self.generate_expression(right);
                        if !right_is_flt && !is_definitely_not_numeric(right, &self.ctx.variables) {
                            // Index carries kind tag (x64: %edx, arm64: w1;
                            // map routing or Phase 1 array slot kinds).
                            if matches!(
                                &**right,
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            ) && is_tag_carrying_read(right, &self.ctx.variables)
                            {
                                let l_skip = self.ctx.next_label();
                                if matches!(self.arch, Architecture::X64) {
                                    self.output
                                        .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    je {}\n", l_skip));
                                } else {
                                    self.output
                                        .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                                    self.output.push_str(&format!("    b.eq {}\n", l_skip));
                                }
                                arch::emit_int_to_float(&mut self.output, self.arch);
                                self.output.push_str(&format!("{}:\n", l_skip));
                            } else {
                                arch::emit_int_to_float(&mut self.output, self.arch);
                            }
                        }
                        arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                    }
                    arch::emit_jump_if_not_zero(&mut self.output, self.arch, target_label);
                    return;
                }

                if let Expr::Number(n) = &**right {
                    self.generate_expression(left);
                    // Strict dynamic check (#39 Phase 2b): float tag on a
                    // tag-carrying read is the runtime half of the mixed
                    // comparison error.
                    if is_tag_carrying_read(left, &self.ctx.variables) {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                        || is_unsigned_expr(right, &self.ctx.variables);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        *op,
                        false,
                        target_label,
                        unsigned,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**right {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(left);
                        // Strict dynamic check (#39 Phase 2b): float tag on a
                        // tag-carrying read is the runtime half of the mixed
                        // comparison error.
                        if is_tag_carrying_read(left, &self.ctx.variables) {
                            arch::emit_mixed_float_check(&mut self.output, self.arch);
                        }
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                            || is_unsigned_expr(right, &self.ctx.variables);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            *op,
                            false,
                            target_label,
                            unsigned,
                        );
                        return;
                    }
                }
                if let Expr::Number(n) = &**left {
                    self.generate_expression(right);
                    // Strict dynamic check (#39 Phase 2b): float tag on a
                    // tag-carrying read is the runtime half of the mixed
                    // comparison error.
                    if is_tag_carrying_read(right, &self.ctx.variables) {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    arch::emit_cmp_imm(&mut self.output, self.arch, *n as i64);
                    let swapped_op = match op {
                        BinaryOp::Less => BinaryOp::Greater,
                        BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                        BinaryOp::Greater => BinaryOp::Less,
                        BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                        other => *other,
                    };
                    let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                        || is_unsigned_expr(right, &self.ctx.variables);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        swapped_op,
                        false,
                        target_label,
                        unsigned,
                    );
                    return;
                }
                if let Expr::Identifier(var_name) = &**left {
                    if let Some(&VarType::Number(offset)) = self.ctx.variables.get(var_name) {
                        self.generate_expression(right);
                        // Strict dynamic check (#39 Phase 2b): float tag on a
                        // tag-carrying read is the runtime half of the mixed
                        // comparison error.
                        if is_tag_carrying_read(right, &self.ctx.variables) {
                            arch::emit_mixed_float_check(&mut self.output, self.arch);
                        }
                        arch::emit_load_var_to_scratch(&mut self.output, self.arch, offset, false);
                        arch::emit_cmp_reg(&mut self.output, self.arch);
                        let swapped_op = match op {
                            BinaryOp::Less => BinaryOp::Greater,
                            BinaryOp::LessEqual => BinaryOp::GreaterEqual,
                            BinaryOp::Greater => BinaryOp::Less,
                            BinaryOp::GreaterEqual => BinaryOp::LessEqual,
                            other => *other,
                        };
                        let unsigned = is_unsigned_expr(left, &self.ctx.variables)
                            || is_unsigned_expr(right, &self.ctx.variables);
                        arch::emit_cond_jump(
                            &mut self.output,
                            self.arch,
                            swapped_op,
                            false,
                            target_label,
                            unsigned,
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

        // Flow-sensitive type narrowing for Expr::TypeCheck in if condition (e.g. when is StructType)
        let mut type_checks = Vec::new();
        fn extract_type_checks(expr: &Expr, checks: &mut Vec<(String, String)>) {
            match expr {
                Expr::TypeCheck {
                    expr,
                    target,
                    negated: false,
                } => {
                    if let Expr::Identifier(var_name) = expr.as_ref() {
                        checks.push((var_name.clone(), target.clone()));
                    }
                }
                Expr::Binary {
                    left,
                    op: BinaryOp::And,
                    right,
                } => {
                    extract_type_checks(left, checks);
                    extract_type_checks(right, checks);
                }
                _ => {}
            }
        }
        extract_type_checks(condition, &mut type_checks);
        for (var_name, target) in type_checks {
            if let Some(var_type) = self.ctx.variables.get(&var_name) {
                let offset = match var_type {
                    VarType::Number(o)
                    | VarType::Float(o)
                    | VarType::StringOffset(o)
                    | VarType::Array(o)
                    | VarType::Map(o)
                    | VarType::Null(o) => *o,
                    VarType::Struct { offset, .. } | VarType::Interface { offset, .. } => *offset,
                    VarType::StringLabel(_) => 0,
                };
                let matched_struct = self
                    .ctx
                    .structs
                    .keys()
                    .find(|k| {
                        let bare = k.rsplit("::").next().unwrap_or(k);
                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                        *k == &target || bare == target.as_str()
                    })
                    .cloned();
                if let Some(sname) = matched_struct {
                    self.ctx.variables.insert(
                        var_name.clone(),
                        VarType::Struct {
                            struct_name: sname,
                            offset,
                        },
                    );
                }
            }
        }

        for s in then_block {
            self.generate_statement(s);
        }

        let then_variables = self.ctx.variables.clone();
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

        let mut final_variables = initial_variables;
        for (var_name, var_type) in &then_variables {
            if let Some(orig_type) = final_variables.get(var_name) {
                if matches!(orig_type, VarType::Null(_)) && !matches!(var_type, VarType::Null(_)) {
                    final_variables.insert(var_name.clone(), var_type.clone());
                }
            }
        }

        self.output.push_str(&format!("{}:\n", end_label));
        self.ctx.stack_offset = initial_stack_offset;
        self.ctx.variables = final_variables;
    }

    /// A `let` value shape safe to pre-null in a loop entry: literals,
    /// direct struct constructors, and proven-fresh calls/arrays/maps.
    /// Anything else (notably floats and unmarked calls, which may
    /// return ints) is skipped and keeps leaking safely instead of risking
    /// a bad release (issue #80).
    fn loop_let_is_prenullable(&self, value: &Expr) -> bool {
        if is_float_expr(value, &self.ctx.variables) {
            return false;
        }
        match value {
            Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } => true,
            Expr::Call { name, .. } => {
                self.ctx.structs.contains_key(name)
                    || crate::codegen::analysis::call_returns_fresh_value(name, &self.ctx.variables)
                    || is_array_expr(value, &self.ctx.variables)
                    || is_map_expr(value, &self.ctx.variables)
                    || self.get_expr_struct_name(value).is_some()
            }
            _ => false,
        }
    }

    /// Collect `let`-bound names with proven-heap values in a loop body,
    /// recursing into plain blocks but not into nested loops (which
    /// manage their own scopes) or functions (fresh frames). Only names
    /// absent from the entry map qualify: present names rebind outer
    /// slots, which `let` already releases soundly.
    fn collect_loop_heap_lets(
        &self,
        body: &[Stmt],
        entry: &std::collections::HashMap<String, VarType>,
        out: &mut Vec<String>,
    ) {
        for s in body {
            match s.inner_stmt() {
                Stmt::Let { name, value, .. } => {
                    if !entry.contains_key(name)
                        && !out.contains(name)
                        && self.loop_let_is_prenullable(value)
                    {
                        out.push(name.clone());
                    }
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.collect_loop_heap_lets(then_block, entry, out);
                    if let Some(els) = else_block {
                        self.collect_loop_heap_lets(els, entry, out);
                    }
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    self.collect_loop_heap_lets(try_block, entry, out);
                    self.collect_loop_heap_lets(catch_block, entry, out);
                    if let Some(fin) = finally_block {
                        self.collect_loop_heap_lets(fin, entry, out);
                    }
                }
                Stmt::While { .. }
                | Stmt::Repeat { .. }
                | Stmt::For { .. }
                | Stmt::ForEach { .. }
                | Stmt::Function { .. } => {}
                _ => {}
            }
        }
    }

    /// Allocate one slot per name and null it, returning the (name,
    /// offset) pairs. Tracking insertion is left to the caller (after
    /// loop prologue reads, so outer names resolve correctly).
    fn pre_null_loop_vars(&mut self, names: &[String]) -> Vec<(String, i32)> {
        let mut out = Vec::new();
        for name in names {
            arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
            let off = self.ctx.stack_offset;
            arch::emit_load_num(&mut self.output, self.arch, 0);
            arch::emit_store_var(&mut self.output, self.arch, off, self.ctx.stack_offset);
            out.push((name.clone(), off));
        }
        out
    }

    /// Insert pre-nulled slots into tracking so body `let`s reuse (rather
    /// than shadow) them. The `Array` marker is a may-fact (any value
    /// kind can flow in): only the assigned-kind union
    /// (`nullable_heap_vars`) ever counts as proof, never the `VarType`.
    fn track_loop_vars(&mut self, vars: &[(String, i32)]) {
        for (name, off) in vars {
            self.ctx
                .variables
                .insert(name.clone(), VarType::Array(*off));
        }
    }

    /// Drop the pre-nulled temps at loop exit (normal exhaustion and
    /// `break` land here; `return` is covered by scope machinery).
    /// Null-or-valid slots make this unconditionally safe.
    fn release_loop_vars(&mut self, vars: &[(String, i32)]) {
        for (_, off) in vars {
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                *off,
                self.ctx.stack_offset,
                self.os,
            );
        }
    }

    pub(super) fn generate_while(&mut self, condition: &Expr, body: &[Stmt]) {
        let start_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let loop_body_variables = self.ctx.variables.clone();
        // Pre-null loop-fresh heap `let` slots once, before iterations:
        // every rebinding then targets a null-or-valid slot, so the
        // release is always safe (issue #80). Tracking is inserted
        // after the condition so outer reads resolve correctly.
        let mut prenull_names = Vec::new();
        self.collect_loop_heap_lets(body, &loop_body_variables, &mut prenull_names);
        let prenulled = self.pre_null_loop_vars(&prenull_names);
        let loop_body_stack_offset = self.ctx.stack_offset;

        self.ctx.push_loop(
            start_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        self.generate_condition_jump_if_false(condition, &end_label);
        self.track_loop_vars(&prenulled);

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
        self.release_loop_vars(&prenulled);

        self.ctx.pop_loop();
    }

    pub(super) fn generate_repeat(&mut self, body: &[Stmt]) {
        let start_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let loop_body_variables = self.ctx.variables.clone();
        let mut prenull_names = Vec::new();
        self.collect_loop_heap_lets(body, &loop_body_variables, &mut prenull_names);
        let prenulled = self.pre_null_loop_vars(&prenull_names);
        let loop_body_stack_offset = self.ctx.stack_offset;

        self.ctx.push_loop(
            start_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        self.track_loop_vars(&prenulled);

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
        self.release_loop_vars(&prenulled);

        self.ctx.pop_loop();
    }

    pub(super) fn generate_for(
        &mut self,
        var: &str,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
        body: &[Stmt],
    ) {
        let var = var.to_string();
        // alya-lang/alya#163: the loop rebinds its variable.
        self.clear_map_struct_markers(&var);
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
        let loop_body_variables = self.ctx.variables.clone();
        let mut prenull_names = Vec::new();
        self.collect_loop_heap_lets(body, &loop_body_variables, &mut prenull_names);
        let prenulled = self.pre_null_loop_vars(&prenull_names);
        let loop_body_stack_offset = self.ctx.stack_offset;

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
        // Half-open `..` exits when var reaches end (`>=`); inclusive `..=`
        // exits past it (`>`). Chapter 06 §1.3.
        // B4: u64 bounds compare unsigned.
        let bound_unsigned = is_unsigned_expr(start, &self.ctx.variables)
            || is_unsigned_expr(end, &self.ctx.variables);
        if inclusive {
            arch::emit_compare_and_jump_if_greater(
                &mut self.output,
                self.arch,
                &end_label,
                bound_unsigned,
            );
        } else {
            arch::emit_compare_and_jump_if_greater_or_equal(
                &mut self.output,
                self.arch,
                &end_label,
                bound_unsigned,
            );
        }
        self.track_loop_vars(&prenulled);

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
        self.release_loop_vars(&prenulled);

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
        // alya-lang/alya#163: the loop rebinds its variables.
        self.clear_map_struct_markers(&var);
        if let Some(vv) = value_var {
            self.clear_map_struct_markers(vv);
        }
        let is_str_iter = is_string_expr(iterable, &self.ctx.variables);
        if is_str_iter {
            let initial_stack_offset = self.ctx.stack_offset;
            self.generate_expression(iterable);
            arch::emit_push_temp(&mut self.output, self.arch);
            // B2: iterate decoded rune codepoints (spec ch.21 §1.4), not
            // 1-char strings: the temp holds ints like `bytes()` does.
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "string_codepoints",
                1,
                initial_stack_offset,
                self.os,
            );
        } else {
            self.generate_expression(iterable);
        }
        arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
        let arr_offset = self.ctx.stack_offset;

        // A fresh-owned iterable (call result, literal, or the
        // `string_codepoints` array built for string iteration) has no owning
        // variable: the loop must release it or it leaks every iteration
        // (alya-lang/alya#79). Calls qualify only with a freshness
        // marker (or as direct struct constructors): a call may return
        // a borrow, which must not be dropped. Named variables stay
        // borrowed; their scope owns the release.
        let owns_iter_temp = is_str_iter
            || matches!(
                iterable,
                Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. }
            )
            || match iterable {
                Expr::Call { name, .. } => {
                    self.ctx.structs.contains_key(name)
                        || crate::codegen::analysis::call_returns_fresh_value(
                            name,
                            &self.ctx.variables,
                        )
                }
                _ => false,
            };
        // Registered as a plain tracked variable so `return` inside the
        // body drops it through the normal scope machinery; removed again
        // after the loop to avoid a second release at scope end.
        let iter_tmp_var: Option<String> = if owns_iter_temp {
            let hid = format!(
                "__foreach_tmp_{}",
                self.ctx
                    .next_label()
                    .trim_start_matches('.')
                    .trim_start_matches('L')
            );
            self.ctx
                .variables
                .insert(hid.clone(), VarType::Array(arr_offset));
            Some(hid)
        } else {
            None
        };

        arch::emit_load_num(&mut self.output, self.arch, 0);
        arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
        let idx_offset = self.ctx.stack_offset;

        let is_str = is_str_iter || is_string_array(iterable, &self.ctx.variables);
        let is_flt = !is_str_iter && is_float_array(iterable, &self.ctx.variables);

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
            arch::emit_load_var(&mut self.output, self.arch, ch_offset, initial_stack_offset);
            arch::emit_push_temp(&mut self.output, self.arch);
            arch::emit_load_num(&mut self.output, self.arch, 5000);
            arch::emit_push_temp(&mut self.output, self.arch);

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
                false,
            );

            arch::emit_store_var(
                &mut self.output,
                self.arch,
                var_offset,
                self.ctx.stack_offset,
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
            arch::emit_jump(&mut self.output, self.arch, &start_label);
            self.output.push_str(&format!("{}:\n", end_label));

            self.ctx.pop_loop();
            if let Some(hid) = &iter_tmp_var {
                self.ctx.variables.remove(hid);
            }
            return;
        }

        let is_map = is_map_expr(iterable, &self.ctx.variables);
        let is_map_str_val = is_map
            && match iterable {
                Expr::Map(entries) => {
                    // ALL values must be strings: one int among strings
                    // miscompiles the loop value as `%s` (segfault).
                    !entries.is_empty()
                        && entries
                            .iter()
                            .all(|(_, v)| is_string_expr(v, &self.ctx.variables))
                }
                Expr::Identifier(name) => {
                    // Existential markers must be gated by the whole-map
                    // veto (alya-lang/alya#39): one non-string write
                    // anywhere demotes the loop value to Number, where
                    // `say` classifies per value at runtime.
                    self.ctx
                        .variables
                        .keys()
                        .any(|k| k.starts_with(&format!("map_str:{}.", name)))
                        && !self
                            .ctx
                            .variables
                            .contains_key(&format!("map_nonstr:{}", name))
                }
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
        } else if is_str_iter {
            // B2: string iteration yields rune codepoints (spec ch.21
            // §1.4), so the loop var is an int; string arrays stay strings.
            VarType::Number(0)
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
                VarType::Interface { interface_name, .. } => VarType::Interface {
                    interface_name: interface_name.clone(),
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
                | VarType::Struct { offset, .. }
                | VarType::Interface { offset, .. },
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
            } else if is_str_iter {
                // B2: two-var string iteration binds index + codepoint.
                VarType::Number(0)
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
                    // Mixed-kind fields suppress the bare-global marker:
                    // no single static kind serves every holder
                    // (alya-lang/alya#131).
                    let kind_mixed = self
                        .ctx
                        .variables
                        .contains_key(&format!("struct_field_mixed:{}", fname));
                    if !kind_mixed
                        && (self
                            .ctx
                            .variables
                            .contains_key(&format!("struct_field_str:{}.{}", sname, fname))
                            || self
                                .ctx
                                .variables
                                .contains_key(&format!("struct_field_str:{}", fname)))
                    {
                        self.ctx
                            .variables
                            .insert(field_key, VarType::StringOffset(0));
                    } else if !kind_mixed
                        && (self
                            .ctx
                            .variables
                            .contains_key(&format!("struct_field_flt:{}.{}", sname, fname))
                            || self
                                .ctx
                                .variables
                                .contains_key(&format!("struct_field_flt:{}", fname)))
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
        let loop_body_variables = self.ctx.variables.clone();
        let mut prenull_names = Vec::new();
        self.collect_loop_heap_lets(body, &loop_body_variables, &mut prenull_names);
        let prenulled = self.pre_null_loop_vars(&prenull_names);
        let loop_body_stack_offset = self.ctx.stack_offset;

        self.ctx.push_loop(
            step_label.clone(),
            end_label.clone(),
            loop_body_stack_offset,
        );

        self.output.push_str(&format!("{}:\n", start_label));
        // The element-receiving slot is float-typed exactly when the
        // static var type is Float (no map/struct/string iteration):
        // mixed elements then convert at the load boundary (Phase 1,
        // #39) instead of reinterpreting raw bits.
        let elem_is_float = inferred_struct_type.is_none() && !is_str && !is_map && is_flt;
        // Map values convert the same way when a value variable exists:
        // Number slots truncate float entries, Float slots widen int
        // entries, String slots move raw (all-string maps only; mixed
        // maps demote to Number via the map_nonstr veto above).
        let map_val_is_float =
            value_var.is_some_and(|v| matches!(self.ctx.variables.get(v), Some(VarType::Float(_))));
        let map_val_is_string = value_var
            .is_some_and(|v| matches!(self.ctx.variables.get(v), Some(VarType::StringOffset(_))));
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
            elem_is_float,
            map_val_is_float,
            map_val_is_string,
        );
        self.track_loop_vars(&prenulled);

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
        // Normal exhaustion and `break` both land here with the iterable
        // slot still live: drop the owned temp (alya-lang/alya#79).
        // `return` paths never reach this label; they are covered by the
        // tracked temp variable above.
        if owns_iter_temp {
            // Fresh-owned iterable temp: release stays probed (the slot
            // is offset-addressed; only the union proves a name).
            arch::emit_rc_release_stack(
                &mut self.output,
                self.arch,
                arr_offset,
                self.ctx.stack_offset,
                self.os,
            );
        }
        self.release_loop_vars(&prenulled);

        self.ctx.pop_loop();
        if let Some(hid) = iter_tmp_var {
            self.ctx.variables.remove(&hid);
        }
    }

    /// Index of the string `message` field when `expr` is a struct
    /// carrying one (e.g. `SocketError { message }`): uncaught throws
    /// print this text instead of the raw struct pointer. `None` keeps
    /// the legacy path (structs without a string message still throw
    /// the value itself, for `catch` to bind).
    fn struct_message_field_idx(&self, expr: &Expr) -> Option<usize> {
        let sname = match expr {
            Expr::StructInit { name, .. } => name.clone(),
            _ => self.get_expr_struct_name(expr)?,
        };
        let bare = sname.rsplit("::").next().unwrap_or(&sname);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let sdef = self
            .ctx
            .structs
            .get(&sname)
            .or_else(|| self.ctx.structs.get(bare))?;
        let idx = sdef.fields.iter().position(|f| f == "message")?;
        match sdef.field_types.get(idx).and_then(|t| t.as_deref()) {
            Some("string") | Some("str") => Some(idx),
            _ => None,
        }
    }

    pub(crate) fn generate_throw(&mut self, opt_expr: Option<&Expr>) {
        if let Some(expr) = opt_expr {
            // fn_throw(value, msg): msg carries struct `message` text
            // (or null); the runtime records both in the thread's block,
            // so no global is involved and stale messages are impossible.
            // The struct value itself is allocated once into a temp slot.
            if self.struct_message_field_idx(expr).is_some() {
                self.generate_expression(expr);
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                let tmp = self.ctx.stack_offset;
                arch::emit_store_var(&mut self.output, self.arch, tmp, self.ctx.stack_offset);
                let msg_idx = self.struct_message_field_idx(expr).unwrap_or(0);
                arch::emit_load_var(&mut self.output, self.arch, tmp, self.ctx.stack_offset);
                arch::emit_push_temp(&mut self.output, self.arch);
                arch::emit_load_var(&mut self.output, self.arch, tmp, self.ctx.stack_offset);
                arch::emit_struct_field_get(&mut self.output, self.arch, msg_idx);
                arch::emit_push_temp(&mut self.output, self.arch);
            } else {
                let is_struct = self.get_expr_struct_name(expr).is_some()
                    || match expr {
                        Expr::StructInit { .. } => true,
                        Expr::Identifier(id) => {
                            matches!(self.ctx.variables.get(id), Some(VarType::Struct { .. }))
                                || self
                                    .ctx
                                    .variables
                                    .contains_key(&format!("is_catch_var:{}", id))
                        }
                        _ => false,
                    };
                if is_string_expr(expr, &self.ctx.variables) || is_struct {
                    self.generate_expression(expr);
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
                }
                arch::emit_push_temp(&mut self.output, self.arch);
                arch::emit_load_num(&mut self.output, self.arch, 0);
                arch::emit_push_temp(&mut self.output, self.arch);
            }
            arch::emit_function_call(
                &mut self.output,
                self.arch,
                "throw",
                2,
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
                arch::emit_try_begin(
                    &mut self.output,
                    self.arch,
                    &catch_label,
                    self.ctx.stack_offset,
                    self.os,
                );
                for s in try_block {
                    self.generate_statement(s);
                }
                let try_delta = self.ctx.stack_offset - saved_stack_offset;
                arch::emit_try_end(
                    &mut self.output,
                    self.arch,
                    &finally_normal_label,
                    try_delta,
                    self.ctx.stack_offset,
                    self.os,
                );

                // Catch block
                self.ctx.stack_offset = saved_stack_offset;
                self.ctx.variables = saved_variables.clone();
                arch::emit_catch_begin(&mut self.output, self.arch, &catch_label);

                // Temporary try handler so that errors inside catch run finally and rethrow
                arch::emit_try_begin(
                    &mut self.output,
                    self.arch,
                    &finally_rethrow_label,
                    self.ctx.stack_offset,
                    self.os,
                );

                if let Some(name) = catch_var {
                    let name = name.to_string();
                    arch::emit_catch_load_err(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                    arch::emit_allocate_var(
                        &mut self.output,
                        self.arch,
                        &mut self.ctx.stack_offset,
                    );
                    self.ctx
                        .variables
                        .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
                    self.ctx
                        .variables
                        .insert(format!("is_catch_var:{}", name), VarType::Number(0));
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
                    self.ctx.stack_offset,
                    self.os,
                );
            } else {
                // Try block WITHOUT catch (only finally)
                arch::emit_try_begin(
                    &mut self.output,
                    self.arch,
                    &finally_rethrow_label,
                    self.ctx.stack_offset,
                    self.os,
                );
                for s in try_block {
                    self.generate_statement(s);
                }
                let try_delta = self.ctx.stack_offset - saved_stack_offset;
                arch::emit_try_end(
                    &mut self.output,
                    self.arch,
                    &finally_normal_label,
                    try_delta,
                    self.ctx.stack_offset,
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
            arch::emit_try_begin(
                &mut self.output,
                self.arch,
                &catch_label,
                self.ctx.stack_offset,
                self.os,
            );

            for s in try_block {
                self.generate_statement(s);
            }

            let try_delta = self.ctx.stack_offset - saved_stack_offset;
            arch::emit_try_end(
                &mut self.output,
                self.arch,
                &end_label,
                try_delta,
                self.ctx.stack_offset,
                self.os,
            );

            // At catch entry, runtime SP has been restored to saved_stack_offset.
            self.ctx.stack_offset = saved_stack_offset;
            self.ctx.variables = saved_variables.clone();
            arch::emit_catch_begin(&mut self.output, self.arch, &catch_label);

            if let Some(name) = catch_var {
                let name = name.to_string();
                arch::emit_catch_load_err(
                    &mut self.output,
                    self.arch,
                    self.ctx.stack_offset,
                    self.os,
                );
                arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
                self.ctx
                    .variables
                    .insert(name.clone(), VarType::StringOffset(self.ctx.stack_offset));
                self.ctx
                    .variables
                    .insert(format!("is_catch_var:{}", name), VarType::Number(0));
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
