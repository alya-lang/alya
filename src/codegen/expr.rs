use super::CodeGen;
use crate::ast::{BinaryOp, Expr};
use crate::codegen::analysis::{
    eq_operand_is_dynamic, escape_string, is_array_expr, is_array_fold_true,
    is_definitely_not_numeric, is_float_expr, is_map_expr, is_map_fold_true, is_null_expr,
    is_number_expr, is_strict_dynamic_op, is_string_expr, is_string_fold_true,
    is_tag_carrying_read, is_unsigned_expr, string_store_needs_dup,
    struct_field_markers_mixed_vars, ternary_arm_carries, typeof_operand_is_repeatable,
    value_kind_tag,
};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::kinds::{KIND_FLOAT, KIND_INT, KIND_STRING, KIND_UNKNOWN};
use crate::codegen::target::{Architecture, OperatingSystem};

impl CodeGen {
    pub(crate) fn generate_expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Null => {
                arch::emit_load_num(&mut self.output, self.arch, 0);
            }
            // Force unwrap is a check-time operation: the value is already
            // non-null (or traps downstream exactly as before, when `!` was
            // erased). Runtime emission is identical to the inner expression.
            Expr::ForceUnwrap(inner) => {
                self.generate_expression(inner);
                // Null trap for heap-typed values (Chapter 19 §1.6), routed
                // through fn_throw so it is catchable like other runtime
                // errors. Scalars (0/false are valid values, not null) and
                // unknown-typed values skip the check (no regression vs the
                // previous silent behavior).
                let heapish = is_null_expr(inner, &self.ctx.variables)
                    || is_string_expr(inner, &self.ctx.variables)
                    || is_array_expr(inner, &self.ctx.variables)
                    || is_map_expr(inner, &self.ctx.variables)
                    || self.get_expr_struct_name(inner).is_some()
                    || self.get_expr_interface_name(inner).is_some();
                if heapish {
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::Equal,
                        false,
                        "alya_error_null_unwrap",
                        false,
                    );
                }
            }
            Expr::Number(n) => {
                arch::emit_load_num(&mut self.output, self.arch, *n as i64);
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
                    let temp_offset = self.temp_offset();
                    self.ctx.stack_offset += temp_offset;

                    self.generate_expression(right);
                    self.ctx.stack_offset -= temp_offset;

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

                if let Some(sname) = self.get_expr_struct_name(left) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let op_str = match op {
                        BinaryOp::Add => Some("+"),
                        BinaryOp::Subtract => Some("-"),
                        BinaryOp::Multiply => Some("*"),
                        BinaryOp::Divide => Some("/"),
                        BinaryOp::Modulo => Some("%"),
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
                        let matched = if self.ctx.functions.contains(&cand1) {
                            Some(cand1.clone())
                        } else if self.ctx.functions.contains(&cand2) {
                            Some(cand2.clone())
                        } else {
                            // Struct-qualified fallback first: aliased imports
                            // duplicate operator overloads under a namespace
                            // prefix that may use either separator
                            // (`tensor__f64x4__...` or `tensor::f64x4__...`),
                            // so a bare `__operator*` suffix match would
                            // lottery-pick across vector types. Normalize
                            // separators before comparing, then prefer the
                            // receiver's own overload; keep the unqualified
                            // search only for structs without a visible
                            // overload.
                            let want1 = format!("__{}__{}{}", sname, "operator", op_sym);
                            let want2 = format!("__{}__{}{}", bare_sname, "operator", op_sym);
                            self.ctx
                                .functions
                                .iter()
                                .find(|f| {
                                    let n = f.replace("::", "__");
                                    n.ends_with(&want1) || n.ends_with(&want2)
                                })
                                .or_else(|| {
                                    self.ctx.functions.iter().find(|f| {
                                        f.ends_with(&format!("__{}{}", "operator", op_sym))
                                    })
                                })
                                .cloned()
                        };
                        if let Some(call_name) = matched {
                            self.generate_expression(&Expr::Call {
                                name: call_name,
                                args: vec![(**left).clone(), (**right).clone()],
                            });
                            return;
                        }
                        // Derived comparisons (Chapter 20 §1.3): `!=`, `<=`,
                        // `>` and `>=` need no explicit overloads. Each side
                        // is evaluated exactly once.
                        //   a != b  =>  not (a == b)   (requires ==)
                        //   a <= b  =>  not (b < a)    (requires <)
                        //   a > b   =>  (b < a)        (requires <)
                        //   a >= b  =>  not (a < b)    (requires <)
                        let derived: Option<(bool, &str, bool)> = match op {
                            BinaryOp::NotEqual => Some((false, "==", true)),
                            BinaryOp::LessEqual => Some((true, "<", true)),
                            BinaryOp::Greater => Some((true, "<", false)),
                            BinaryOp::GreaterEqual => Some((false, "<", true)),
                            _ => None,
                        };
                        if let Some((swap, base_sym, negate)) = derived {
                            let base1 = format!("{}__{}{}", sname, "operator", base_sym);
                            let base2 = format!("{}__{}{}", bare_sname, "operator", base_sym);
                            let base_call = if self.ctx.functions.contains(&base1) {
                                Some(base1)
                            } else if self.ctx.functions.contains(&base2) {
                                Some(base2)
                            } else {
                                None
                            };
                            if let Some(base_name) = base_call {
                                let (first, second) = if swap {
                                    ((**right).clone(), (**left).clone())
                                } else {
                                    ((**left).clone(), (**right).clone())
                                };
                                let cmp = Expr::Call {
                                    name: base_name,
                                    args: vec![first, second],
                                };
                                if negate {
                                    self.generate_expression(&Expr::Unary {
                                        op: crate::ast::UnaryOp::Not,
                                        expr: Box::new(cmp),
                                    });
                                } else {
                                    self.generate_expression(&cmp);
                                }
                                return;
                            }
                        }
                    }
                }

                // Dynamic equality (issues #41/#44): operands that are not
                // both proven strings are verified at runtime (string ->
                // content compare, otherwise -> word compare). Static
                // string markings are trusted only when BOTH sides carry
                // them: bare-name inference markers can collide across
                // functions (e.g. `expected: string` in `assert_str_eq`
                // mis-marks an int `expected` elsewhere), so a singly
                // marked side is re-verified instead of trusted.
                // Literals, floats, maps, arrays, nulls, and proven
                // integers keep their existing paths exactly.
                if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual)
                    && !left_is_num
                    && !right_is_num
                {
                    let left_str = is_string_expr(left, &self.ctx.variables);
                    let right_str = is_string_expr(right, &self.ctx.variables);
                    if left_str && right_str {
                        self.generate_string_equality(left, right, *op);
                        return;
                    }
                    if left_str
                        || right_str
                        || (!is_float_expr(left, &self.ctx.variables)
                            && !is_float_expr(right, &self.ctx.variables)
                            && !is_map_expr(left, &self.ctx.variables)
                            && !is_map_expr(right, &self.ctx.variables)
                            && !is_array_expr(left, &self.ctx.variables)
                            && !is_array_expr(right, &self.ctx.variables)
                            && !is_null_expr(left, &self.ctx.variables)
                            && !is_null_expr(right, &self.ctx.variables)
                            && (eq_operand_is_dynamic(left, &self.ctx.variables)
                                || eq_operand_is_dynamic(right, &self.ctx.variables)))
                    {
                        self.generate_dynamic_equality(left, right, *op);
                        return;
                    }
                }

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
                        if !left_is_float && !is_definitely_not_numeric(left, &self.ctx.variables) {
                            // Index carries kind tag alongside the value
                            // (x64: %edx, arm64: w1): skip int->float
                            // when the value is already a float.
                            // Array loads carry slot kinds (Phase 1, #39).
                            if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && matches!(
                                    &**left,
                                    Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                                )
                                && is_tag_carrying_read(left, &self.ctx.variables)
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
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n);
                        return;
                    }
                    if let Expr::Number(n) = &**right {
                        self.generate_expression(left);
                        if !left_is_float && !is_definitely_not_numeric(left, &self.ctx.variables) {
                            // Index carries kind tag (x64: %edx, arm64: w1).
                            // Array loads carry slot kinds (Phase 1, #39).
                            if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && matches!(
                                    &**left,
                                    Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                                )
                                && is_tag_carrying_read(left, &self.ctx.variables)
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
                        arch::emit_float_binary_op_imm(&mut self.output, self.arch, *op, *n as f64);
                        return;
                    }
                    if let Expr::Identifier(name) = &**right {
                        if let Some(&VarType::Float(offset)) = self.ctx.variables.get(name) {
                            self.generate_expression(left);
                            if !left_is_float
                                && !is_definitely_not_numeric(left, &self.ctx.variables)
                            {
                                // Index carries kind tag (x64: %edx, arm64: w1).
                                // Array loads carry slot kinds (Phase 1, #39).
                                if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                    && matches!(
                                        &**left,
                                        Expr::Index { .. }
                                            | Expr::Ternary { .. }
                                            | Expr::Call { .. }
                                    )
                                    && is_tag_carrying_read(left, &self.ctx.variables)
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
                        // Strict dynamic check (#39 Phase 2b): float tag
                        // on a tag-carrying read is the runtime half of
                        // the mixed-type error.
                        if is_strict_dynamic_op(op)
                            && is_tag_carrying_read(left, &self.ctx.variables)
                        {
                            arch::emit_mixed_float_check(&mut self.output, self.arch);
                        }
                        arch::emit_binary_op_imm(
                            &mut self.output,
                            self.arch,
                            *op,
                            *n as i64,
                            is_unsigned_expr(left, &self.ctx.variables)
                                || is_unsigned_expr(right, &self.ctx.variables),
                        );
                        return;
                    }
                    if let Expr::Identifier(name) = &**right {
                        if let Some(&VarType::Number(offset)) = self.ctx.variables.get(name) {
                            self.generate_expression(left);
                            if is_strict_dynamic_op(op)
                                && is_tag_carrying_read(left, &self.ctx.variables)
                            {
                                arch::emit_mixed_float_check(&mut self.output, self.arch);
                            }
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                offset,
                                false,
                            );
                            arch::emit_binary_op_reg(
                                &mut self.output,
                                self.arch,
                                *op,
                                is_unsigned_expr(left, &self.ctx.variables)
                                    || is_unsigned_expr(right, &self.ctx.variables),
                            );
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
                            if is_strict_dynamic_op(op)
                                && is_tag_carrying_read(right, &self.ctx.variables)
                            {
                                arch::emit_mixed_float_check(&mut self.output, self.arch);
                            }
                            arch::emit_binary_op_imm(
                                &mut self.output,
                                self.arch,
                                *op,
                                *n as i64,
                                is_unsigned_expr(left, &self.ctx.variables)
                                    || is_unsigned_expr(right, &self.ctx.variables),
                            );
                            return;
                        }
                        if let Expr::Identifier(name) = &**left {
                            if let Some(&VarType::Number(offset)) = self.ctx.variables.get(name) {
                                self.generate_expression(right);
                                if is_strict_dynamic_op(op)
                                    && is_tag_carrying_read(right, &self.ctx.variables)
                                {
                                    arch::emit_mixed_float_check(&mut self.output, self.arch);
                                }
                                arch::emit_load_var_to_scratch(
                                    &mut self.output,
                                    self.arch,
                                    offset,
                                    false,
                                );
                                arch::emit_binary_op_reg(
                                    &mut self.output,
                                    self.arch,
                                    *op,
                                    is_unsigned_expr(left, &self.ctx.variables)
                                        || is_unsigned_expr(right, &self.ctx.variables),
                                );
                                return;
                            }
                        }
                    }
                }

                if is_float {
                    self.generate_expression(left);
                    if !left_is_float && !is_definitely_not_numeric(left, &self.ctx.variables) {
                        // Index carries kind tag (x64: %edx, arm64: w1).
                        // Array loads carry slot kinds (Phase 1, #39).
                        if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                            && matches!(
                                left.as_ref(),
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            )
                            && is_tag_carrying_read(left, &self.ctx.variables)
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
                    arch::emit_push_temp(&mut self.output, self.arch);

                    let float_temp_offset = match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                    self.ctx.stack_offset += float_temp_offset;
                    self.generate_expression(right);
                    if !right_is_float && !is_definitely_not_numeric(right, &self.ctx.variables) {
                        // Index carries kind tag (x64: %edx, arm64: w1).
                        if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                            && matches!(
                                right.as_ref(),
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            )
                            && is_tag_carrying_read(right, &self.ctx.variables)
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
                    self.ctx.stack_offset -= float_temp_offset;

                    arch::emit_float_binary_op(&mut self.output, self.arch, *op);
                } else {
                    self.generate_expression(left);
                    // Strict dynamic check (alya-lang/alya#39 Phase 2b):
                    // the static checker rejects provably-mixed reads; a
                    // float tag on a tag-carrying read is the runtime
                    // half of that error. Int/unknown tags proceed.
                    if is_strict_dynamic_op(op) && is_tag_carrying_read(left, &self.ctx.variables) {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    arch::emit_push_temp(&mut self.output, self.arch);

                    let temp_offset = self.temp_offset();
                    self.ctx.stack_offset += temp_offset;
                    self.generate_expression(right);
                    if is_strict_dynamic_op(op) && is_tag_carrying_read(right, &self.ctx.variables)
                    {
                        arch::emit_mixed_float_check(&mut self.output, self.arch);
                    }
                    self.ctx.stack_offset -= temp_offset;
                    arch::emit_binary_op(
                        &mut self.output,
                        self.arch,
                        *op,
                        is_unsigned_expr(left, &self.ctx.variables)
                            || is_unsigned_expr(right, &self.ctx.variables),
                    );
                }
            }
            Expr::Unary { op, expr } => {
                if *op == crate::ast::UnaryOp::Negate {
                    if let Some(sname) = self.get_expr_struct_name(expr) {
                        let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                        let cand1 = format!("{}__{}", sname, "operator-neg");
                        let cand2 = format!("{}__{}", bare_sname, "operator-neg");
                        let matched = if self.ctx.functions.contains(&cand1) {
                            Some(cand1)
                        } else if self.ctx.functions.contains(&cand2) {
                            Some(cand2)
                        } else {
                            self.ctx
                                .functions
                                .iter()
                                .find(|f| f.ends_with("__operator-neg"))
                                .cloned()
                        };
                        if let Some(call_name) = matched {
                            self.generate_expression(&Expr::Call {
                                name: call_name,
                                args: vec![(**expr).clone()],
                            });
                            return;
                        }
                    }
                }
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

                // Tag-carrying ternary results (Phase 2b, #39): when this
                // ternary is not statically float but an arm delivers a
                // tag, every other arm materializes its static kind so
                // the join always carries the taken arm's tag in the tag
                // register (x64: %edx, arm64: w1). Tag-carrying arms
                // need no emission; unknown shapes record 0 (status quo).
                let ternary_carries = !is_flt
                    && (ternary_arm_carries(then_branch, &self.ctx.variables)
                        || ternary_arm_carries(else_branch, &self.ctx.variables));

                self.generate_expression(then_branch);
                if ternary_carries && !ternary_arm_carries(then_branch, &self.ctx.variables) {
                    let tag: i64 = match &**then_branch {
                        Expr::Number(_) => KIND_INT,
                        Expr::String(_) => KIND_STRING,
                        _ => KIND_UNKNOWN,
                    };
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str(&format!("    movl ${}, %edx\n", tag));
                        }
                        Architecture::ARM64 => {
                            self.output.push_str(&format!("    mov w1, #{}\n", tag));
                        }
                    }
                }
                if is_flt
                    && !is_float_expr(then_branch, &self.ctx.variables)
                    && !is_definitely_not_numeric(then_branch, &self.ctx.variables)
                {
                    // Kind-carrying reads already holding float bits skip
                    // the conversion (map routing or Phase 1 array kinds).
                    let already_float =
                        matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                            && matches!(
                                &**then_branch,
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            )
                            && is_tag_carrying_read(then_branch, &self.ctx.variables);
                    if already_float {
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
                arch::emit_jump(&mut self.output, self.arch, &end_label);

                self.output.push_str(&format!("{}:\n", else_label));
                self.generate_expression(else_branch);
                if ternary_carries && !ternary_arm_carries(else_branch, &self.ctx.variables) {
                    let tag: i64 = match &**else_branch {
                        Expr::Number(_) => KIND_INT,
                        Expr::String(_) => KIND_STRING,
                        _ => KIND_UNKNOWN,
                    };
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str(&format!("    movl ${}, %edx\n", tag));
                        }
                        Architecture::ARM64 => {
                            self.output.push_str(&format!("    mov w1, #{}\n", tag));
                        }
                    }
                }
                if is_flt
                    && !is_float_expr(else_branch, &self.ctx.variables)
                    && !is_definitely_not_numeric(else_branch, &self.ctx.variables)
                {
                    // Kind-carrying reads already holding float bits skip
                    // the conversion (map routing or Phase 1 array kinds).
                    let already_float =
                        matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                            && matches!(
                                &**else_branch,
                                Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                            )
                            && is_tag_carrying_read(else_branch, &self.ctx.variables);
                    if already_float {
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
                            false,
                        );
                        if !is_definitely_not_numeric(value, &self.ctx.variables) {
                            // Kind-carrying reads already holding float
                            // bits skip the conversion.
                            let already_float =
                                matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                    && matches!(
                                        &**value,
                                        Expr::Index { .. }
                                            | Expr::Ternary { .. }
                                            | Expr::Call { .. }
                                    )
                                    && is_tag_carrying_read(value, &self.ctx.variables);
                            if already_float {
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
                        arch::emit_jump(&mut self.output, self.arch, &end_label);
                    } else {
                        arch::emit_jump(&mut self.output, self.arch, &end_label);
                    }

                    self.output.push_str(&format!("{}:\n", default_label));
                    self.generate_expression(default);
                    if !is_float_expr(default, &self.ctx.variables)
                        && !is_definitely_not_numeric(default, &self.ctx.variables)
                    {
                        // Kind-carrying reads already holding float bits skip
                        // the conversion (map routing or Phase 1 array kinds).
                        let already_float =
                            matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && matches!(
                                    &**default,
                                    Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                                )
                                && is_tag_carrying_read(default, &self.ctx.variables);
                        if already_float {
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
                        false,
                    );
                    self.generate_expression(default);
                    self.output.push_str(&format!("{}:\n", end_label));
                }
            }

            Expr::Call { name, args } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                // Measurement for alya-lang/alya#55-C (demand side): one
                // record per emitted call into a known user function
                // (builtins, struct constructors, and externs excluded —
                // legacy is their only behavior). A miss whose callee is
                // marked by end of compilation is a forward-reference
                // miss recoverable by a pre-pass; the rest is structural.
                if self.ctx.tag_stats.enabled
                    && (self.ctx.functions.contains(name) || self.ctx.functions.contains(bare))
                {
                    if self
                        .ctx
                        .variables
                        .contains_key(&format!("fn_ret_tagged:{}", name))
                        || self
                            .ctx
                            .variables
                            .contains_key(&format!("fn_ret_tagged:{}", bare))
                    {
                        self.ctx.tag_stats.call_hits += 1;
                    } else {
                        self.ctx.tag_stats.miss_names.push(name.clone());
                        if self.ctx.current_fn_name.is_empty() {
                            self.ctx.tag_stats.miss_top_level += 1;
                        }
                    }
                }
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
                        _ => 8,
                    };
                    self.ctx.stack_offset += temp_offset;

                    for (i, fname) in sdef.fields.iter().enumerate() {
                        let arg = if i < args.len() {
                            &args[i]
                        } else if let Some(Some(def_val)) = sdef.defaults.get(i) {
                            def_val
                        } else {
                            &Expr::Number(0)
                        };
                        let is_flt = is_float_expr(arg, &self.ctx.variables);
                        let is_str = is_string_expr(arg, &self.ctx.variables);
                        let is_arr = is_array_expr(arg, &self.ctx.variables);
                        let is_map = is_map_expr(arg, &self.ctx.variables);
                        // Mixed literal kinds for this field (e.g. `Box{value: 1}`
                        // and `Box{value: "s"}`): no single static marker
                        // serves every instance, so skip global markers and
                        // let reads fall back to runtime classification plus
                        // per-variable keys. Code emission below is unaffected.
                        if !struct_field_markers_mixed_vars(&self.ctx.variables, name, fname) {
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
                                self.ctx.variables.insert(
                                    format!("struct_field_flt:{}", fname),
                                    VarType::Float(0),
                                );
                            } else if is_arr {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}.{}", name, fname),
                                    VarType::Array(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}", fname),
                                    VarType::Array(0),
                                );
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
                        let is_weak = sdef
                            .field_types
                            .get(i)
                            .and_then(|t| t.as_deref())
                            .is_some_and(|t| t.starts_with("weak ") || t == "weak");
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
                    && (matches!(&args[0], Expr::Array(_))
                        || is_array_expr(&args[0], &self.ctx.variables))
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

                if name == "assert_eq"
                    && (args.len() == 2 || args.len() == 3)
                    && !self.user_assert_shadows("assert_eq", args)
                {
                    let eq_expr = Expr::Binary {
                        left: Box::new(args[0].clone()),
                        op: BinaryOp::Equal,
                        right: Box::new(args[1].clone()),
                    };
                    self.generate_expression(&eq_expr);
                    let ok_label = self.ctx.next_label();
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::NotEqual,
                        false,
                        &ok_label,
                        false,
                    );
                    let default_msg =
                        Expr::String("Assertion failed: values are not equal".to_string());
                    let msg_expr = if args.len() == 3 {
                        &args[2]
                    } else {
                        &default_msg
                    };
                    self.generate_throw(Some(msg_expr));
                    self.output.push_str(&format!("{}:\n", ok_label));
                    arch::emit_load_num(&mut self.output, self.arch, 1);
                    return;
                }

                if name == "assert"
                    && (args.len() == 1 || args.len() == 2)
                    && !self.user_assert_shadows("assert", args)
                {
                    self.generate_expression(&args[0]);
                    let ok_label = self.ctx.next_label();
                    arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                    arch::emit_cond_jump(
                        &mut self.output,
                        self.arch,
                        BinaryOp::NotEqual,
                        false,
                        &ok_label,
                        false,
                    );
                    let default_msg = Expr::String("Assertion failed".to_string());
                    let msg_expr = if args.len() == 2 {
                        &args[1]
                    } else {
                        &default_msg
                    };
                    self.generate_throw(Some(msg_expr));
                    self.output.push_str(&format!("{}:\n", ok_label));
                    arch::emit_load_num(&mut self.output, self.arch, 1);
                    return;
                }

                if name == "sizeof" && args.len() == 1 {
                    let tname = match &args[0] {
                        Expr::Identifier(s) => s.as_str(),
                        Expr::String(s) => s.as_str(),
                        _ => "int",
                    };
                    let (size, _) = Self::get_type_size_and_align(tname, &self.ctx.structs);
                    arch::emit_load_num(&mut self.output, self.arch, size);
                    return;
                }

                if name == "alignof" && args.len() == 1 {
                    let tname = match &args[0] {
                        Expr::Identifier(s) => s.as_str(),
                        Expr::String(s) => s.as_str(),
                        _ => "int",
                    };
                    let (_, align) = Self::get_type_size_and_align(tname, &self.ctx.structs);
                    arch::emit_load_num(&mut self.output, self.arch, align);
                    return;
                }

                if name == "typeof" && args.len() == 1 {
                    // Struct/enum names are exact (annotation-driven).
                    if let Some(sname) = self.get_expr_struct_name(&args[0]) {
                        self.generate_expression(&Expr::String(sname));
                        return;
                    }
                    // alya-lang/alya#71: identifiers fold only on strict
                    // (must-) proof; the legacy chain below trusts
                    // may-markings that also cover merely-possible kinds.
                    if let Expr::Identifier(ref id) = args[0] {
                        if self
                            .ctx
                            .variables
                            .contains_key(&format!("param_arr_strict:{}", id))
                        {
                            self.generate_expression(&Expr::String("array".to_string()));
                            return;
                        }
                        if self
                            .ctx
                            .variables
                            .contains_key(&format!("param_map_strict:{}", id))
                        {
                            self.generate_expression(&Expr::String("map".to_string()));
                            return;
                        }
                    } else if let Some(kind) = Self::typeof_literal_kind(&args[0]) {
                        // Literals carry their kind exactly; folding them
                        // keeps the historical constant vocabulary.
                        self.generate_expression(&Expr::String(kind.to_string()));
                        return;
                    }
                    self.emit_typeof_cascade(&args[0]);
                    return;
                }

                let is_struct_receiver = args
                    .first()
                    .and_then(|a| self.get_expr_struct_name(a))
                    .is_some_and(|sname| {
                        let bare = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                        self.ctx.functions.contains(&format!("{}__{}", sname, name))
                            || self.ctx.functions.contains(&format!("{}__{}", bare, name))
                            || self
                                .ctx
                                .functions
                                .iter()
                                .any(|f| f.ends_with(&format!("{}__{}", bare, name)))
                    });

                if name == "get" && args.len() == 2 && !is_struct_receiver {
                    let mut three_args = args.clone();
                    three_args.push(Expr::Number(0));
                    self.generate_expression(&Expr::Call {
                        name: name.clone(),
                        args: three_args,
                    });
                    return;
                }

                if name == "push" && args.len() == 2 && !is_struct_receiver {
                    if let Expr::Identifier(arr_name) = &args[0] {
                        if is_string_expr(&args[1], &self.ctx.variables) {
                            self.ctx
                                .variables
                                .insert(format!("arr_is_str:{}", arr_name), VarType::Number(0));
                        } else {
                            // Mixed content voids the whole-array string
                            // claim (alya-lang/alya#39): readers fall back
                            // to slot-kind dispatch instead of `%s` on a
                            // non-string. Over-marking is safe; the static
                            // claim is what must stay sound.
                            self.ctx
                                .variables
                                .insert(format!("arr_nonstr:{}", arr_name), VarType::Number(0));
                        }
                    }
                    self.generate_expression(&args[0]);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    let temp_offset: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                    self.ctx.stack_offset += temp_offset;
                    self.generate_expression(&args[1]);
                    if self.store_value_needs_retain(&args[1]) {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    // B1: named stores outlive the wrapping ring buffer.
                    if string_store_needs_dup(&args[1], &self.ctx.variables) {
                        arch::emit_str_store(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    self.ctx.stack_offset -= temp_offset;
                    let push_kind = value_kind_tag(&args[1], &self.ctx.variables);
                    arch::emit_array_push(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                        push_kind,
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
                    } else if !is_float_expr(&args[0], &self.ctx.variables)
                        && !is_definitely_not_numeric(&args[0], &self.ctx.variables)
                    {
                        // Kind-carrying reads already holding float bits
                        // must skip the conversion (else cvt mangles them).
                        let already_float =
                            matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && matches!(
                                    &args[0],
                                    Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                                )
                                && is_tag_carrying_read(&args[0], &self.ctx.variables);
                        if already_float {
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
                    if let Some(sname) = self.get_expr_struct_name(&args[0]) {
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
                            self.generate_expression(&Expr::Call {
                                name: call_name,
                                args: vec![args[0].clone()],
                            });
                            return;
                        }
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
                    // Variable-key map reads and (Phase 1, #39) plain
                    // array reads carry the entry kind tag
                    // (x64: %edx, arm64: w1; see fn_get). Floats must go
                    // through str_from_float; everything else uses generic str.
                    if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                        && matches!(
                            &args[0],
                            Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                        )
                        && is_tag_carrying_read(&args[0], &self.ctx.variables)
                    {
                        let initial_stack_offset = self.ctx.stack_offset;
                        self.generate_expression(&args[0]);
                        arch::emit_push_temp(&mut self.output, self.arch);
                        let l_flt = self.ctx.next_label();
                        let l_end = self.ctx.next_label();
                        if matches!(self.arch, Architecture::X64) {
                            self.output
                                .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                            self.output.push_str(&format!("    je {}\n", l_flt));
                        } else {
                            self.output
                                .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                            self.output.push_str(&format!("    b.eq {}\n", l_flt));
                        }
                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "str",
                            1,
                            initial_stack_offset,
                            self.os,
                        );
                        arch::emit_jump(&mut self.output, self.arch, &l_end);
                        self.output.push_str(&format!("{}:\n", l_flt));
                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "str_from_float",
                            1,
                            initial_stack_offset,
                            self.os,
                        );
                        self.output.push_str(&format!("{}:\n", l_end));
                        return;
                    }
                    // B4: unsigned ints use the unsigned decimal converter.
                    if is_unsigned_expr(&args[0], &self.ctx.variables) {
                        let initial_stack_offset = self.ctx.stack_offset;
                        self.generate_expression(&args[0]);
                        arch::emit_push_temp(&mut self.output, self.arch);
                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "str_from_uint",
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
                            }
                            arch::emit_load_var_to_scratch(
                                &mut self.output,
                                self.arch,
                                idx_offset,
                                false,
                            );
                            if matches!(self.arch, Architecture::X64) {
                                self.output.push_str("    mov %rbx, %rcx\n");
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
                    let temp_offset = self.temp_offset();
                    self.ctx.stack_offset += temp_offset;
                    self.generate_expression(&args[1]);
                    self.ctx.stack_offset -= temp_offset;
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
                                    }
                                    arch::emit_load_var_to_scratch(
                                        &mut self.output,
                                        self.arch,
                                        idx_offset,
                                        false,
                                    );
                                    if matches!(self.arch, Architecture::X64) {
                                        self.output.push_str("    mov %rbx, %rcx\n");
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
                            let temp_offset = self.temp_offset();
                            self.ctx.stack_offset += temp_offset;
                            self.generate_expression(&inner_args[1]);
                            self.ctx.stack_offset -= temp_offset;
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
                                    }
                                    arch::emit_load_var_to_scratch(
                                        &mut self.output,
                                        self.arch,
                                        idx_offset,
                                        false,
                                    );
                                    if matches!(self.arch, Architecture::X64) {
                                        self.output.push_str("    mov %rbx, %rcx\n");
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
                            let temp_offset = self.temp_offset();
                            self.ctx.stack_offset += temp_offset;
                            self.generate_expression(index);
                            self.ctx.stack_offset -= temp_offset;
                            let done_label = self.ctx.next_label();
                            arch::emit_char_code_at(&mut self.output, self.arch, &done_label);
                            return;
                        }
                    }
                }

                let mut resolved_name = name.clone();
                let mut actual_args = args.clone();

                // Builtin `spawn` keyword (concurrency): the parser desugars
                // `spawn f(args)` into a two-argument `spawn(fn, arg)` call
                // shaped exactly like the green-fiber scheduler entry point.
                // Only an exact bare `spawn` in scope (user-defined) wins;
                // qualified variants (`sy::spawn`) cannot satisfy a bare
                // call, so they never block the builtin. Otherwise resolve
                // directly to the always-linked fiber runtime symbol
                // (`fn___native_fiber_spawn`) instead of an unresolvable
                // `fn_spawn` stub.
                if name == "spawn" && actual_args.len() == 2 {
                    let user_spawn = self.ctx.functions.contains("spawn")
                        || self.ctx.extern_functions.contains_key("spawn")
                        || self.ctx.variables.contains_key("spawn");
                    if !user_spawn {
                        resolved_name = "__native_fiber_spawn".to_string();
                    }
                }

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
                    Some(Expr::Index { array, .. }) => match &**array {
                        Expr::Identifier(type_name) => Some((None, type_name.clone())),
                        Expr::FieldAccess { object, field } => {
                            if let Expr::Identifier(mod_name) = &**object {
                                Some((Some(mod_name.clone()), field.clone()))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
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
                        || mod_opt.as_ref().is_some_and(|m| {
                            self.ctx
                                .structs
                                .contains_key(&format!("{}::{}", m, type_name))
                        });
                    let is_var = self.ctx.variables.contains_key(&type_name);
                    let is_mod_var = mod_opt
                        .as_ref()
                        .is_some_and(|m| self.ctx.variables.contains_key(m));

                    let valid_target = if mod_opt.is_some() {
                        is_struct && !is_mod_var
                    } else {
                        (is_struct
                            || self.ctx.functions.contains(&mangled1)
                            || self.ctx.functions.contains(&mangled_single)
                            || self.ctx.functions.contains(&mangled2)
                            || self.ctx.functions.contains(&mangled3)
                            || self.ctx.functions.iter().any(|f| {
                                f.ends_with(&format!("__{}", mangled1))
                                    || f.ends_with(&format!("::{}", mangled1))
                            }))
                            && !is_var
                    };

                    if valid_target {
                        if self.ctx.functions.contains(&mangled2) {
                            resolved_name = mangled2;
                        } else if self.ctx.functions.contains(&mangled3) {
                            resolved_name = mangled3;
                        } else if self.ctx.functions.contains(&mangled1) {
                            resolved_name = mangled1;
                        } else if self.ctx.functions.contains(&mangled_single) {
                            resolved_name = mangled_single;
                        } else if let Some(matched) = self.ctx.functions.iter().find(|f| {
                            f.ends_with(&format!("__{}", mangled1))
                                || f.ends_with(&format!("::{}", mangled1))
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
                        let flattened = crate::codegen::get_interface_flattened_methods(
                            &iname,
                            &self.ctx.interfaces,
                        );
                        if let Some((method_idx, _)) =
                            flattened.iter().enumerate().find(|(_, m)| m.name == *name)
                        {
                            let initial_stack_offset = self.ctx.stack_offset;
                            let word_size: i32 = match self.arch {
                                Architecture::ARM64 => 16,
                                _ => 8,
                            };

                            if matches!(self.arch, Architecture::ARM64) {
                                // Evaluate receiver: load concrete instance data_ptr (offset 0 of fat pointer)
                                self.generate_expression(first_arg);
                                self.output.push_str("    ldr x0, [x0]\n");
                                arch::emit_push_temp(&mut self.output, self.arch);

                                // Evaluate remaining arguments
                                for (idx, arg) in actual_args.iter().skip(1).enumerate() {
                                    self.ctx.stack_offset =
                                        initial_stack_offset + ((idx + 1) as i32 * word_size);
                                    self.generate_expression(arg);
                                    arch::emit_push_temp(&mut self.output, self.arch);
                                }
                                self.ctx.stack_offset = initial_stack_offset;

                                // Load method function pointer from vtable:
                                self.generate_expression(first_arg);
                                self.output.push_str("    ldr x16, [x0, #8]\n");
                                self.output.push_str(&format!(
                                    "    ldr x16, [x16, #{}]\n",
                                    (method_idx + 1) * 8
                                ));

                                // Call function pointer
                                arch::arm64::control::emit_call_target(
                                    &mut self.output,
                                    "x16",
                                    actual_args.len(),
                                );
                            } else {
                                // Evaluate receiver: load concrete instance data_ptr (offset 0 of fat pointer)
                                self.generate_expression(first_arg);
                                self.output.push_str("    movq (%rax), %rax\n");
                                arch::emit_push_temp(&mut self.output, self.arch);

                                // Evaluate remaining arguments
                                for (idx, arg) in actual_args.iter().skip(1).enumerate() {
                                    self.ctx.stack_offset =
                                        initial_stack_offset + ((idx + 1) as i32 * word_size);
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
                                self.output.push_str(&format!(
                                    "    movq {}(%r11), %r11\n",
                                    (method_idx + 1) * 8
                                ));

                                // 4. Call function pointer
                                arch::x64::control::emit_call_target(
                                    &mut self.output,
                                    "*%r11",
                                    actual_args.len(),
                                    initial_stack_offset,
                                    self.os,
                                );
                            }

                            let is_flt_ret = flattened[method_idx]
                                .return_type
                                .as_deref()
                                .is_some_and(|rt| rt == "float" || rt == "f64")
                                || self
                                    .ctx
                                    .variables
                                    .contains_key(&format!("fn_ret_flt:{}", name));
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
                            // Normalize separators: aliased imports duplicate
                            // methods under a namespace prefix that may use
                            // `::` (`tensor::f64x4__min`), which never matches
                            // a `__`-style suffix literally.
                            let n = f.replace("::", "__");
                            n.ends_with(&suffix1)
                                || n.ends_with(&suffix2)
                                || n.ends_with(&suffix3.replace("::", "__"))
                                || n.ends_with(&suffix4.replace("::", "__"))
                        }) {
                            resolved_name = matched.clone();
                        }
                    }
                }

                // 2b. Self-delegation guard: a bare `name(self, ...)` call
                // inside the resolved method itself, with all-identifier
                // args, re-enters this same body with identical values and
                // never terminates. When a same-named free function with
                // compatible arity exists, the author meant delegation
                // (e.g. `Regex.is_match` delegating to the free
                // `is_match`): prefer it. Genuine recursion (changed args,
                // non-identifier args, or no free target) is untouched, as
                // are explicitly qualified calls (`Type.method(...)`),
                // which never reach this branch with a bare name.
                if !self.ctx.current_fn_name.is_empty()
                    && !name.contains("::")
                    && !name.contains("__")
                    && !name.contains('.')
                    && actual_args.iter().all(|a| matches!(a, Expr::Identifier(_)))
                {
                    let norm = |s: &str| s.replace("::", "__");
                    let cur = norm(&self.ctx.current_fn_name);
                    let tgt = norm(&resolved_name);
                    let same_target = tgt == cur
                        || tgt.ends_with(&format!("__{}", cur))
                        || cur.ends_with(&format!("__{}", tgt));
                    if same_target {
                        let arity_ok =
                            self.ctx
                                .fn_arities
                                .get(name.as_str())
                                .is_some_and(|(req, total)| {
                                    actual_args.len() >= *req && actual_args.len() <= *total
                                });
                        if arity_ok {
                            if let Some(free_fn) =
                                Self::find_free_function(&self.ctx.functions, name, &cur)
                            {
                                resolved_name = free_fn;
                            }
                        }
                    }
                }

                // 3. Direct namespace / mangled name: Point::create -> Point__create
                if resolved_name.contains("::") {
                    let mangled = if resolved_name.starts_with("_Alya_") {
                        resolved_name.replace("::", "_")
                    } else {
                        resolved_name.replace("::", "__")
                    };
                    if self.ctx.functions.contains(&mangled) {
                        resolved_name = mangled;
                    }
                }

                let call_name_str = if (resolved_name == "substring" || resolved_name == "substr")
                    && actual_args.len() == 2
                {
                    actual_args.push(Expr::Number(-1));
                    "substring".to_string()
                } else if resolved_name == "substr" {
                    "substring".to_string()
                } else if resolved_name == "length" || resolved_name == "byte_length" {
                    "len".to_string()
                } else if resolved_name == "to_upper" {
                    "upper".to_string()
                } else if resolved_name == "to_lower" {
                    "lower".to_string()
                } else if (resolved_name == "contains" || resolved_name == "has")
                    && actual_args.len() == 2
                    && is_map_expr(&actual_args[0], &self.ctx.variables)
                {
                    "has".to_string()
                } else {
                    resolved_name
                };
                // 3b. Bare-global fallback for method-ambiguous names left
                // unprefixed by aliased imports (see `is_method_ambiguous`
                // in the parser): a bare `sum` call must still reach the
                // namespaced `tensor::sum` global rather than emitting an
                // undefined `fn_sum` symbol. Among suffix matches prefer the
                // fewest qualifier segments so the global wins over a
                // same-bare method (`tensor::sum` over
                // `tensor::Tensor__sum`). Fire ONLY for namespaced hits
                // (containing `::`, the mark of aliasing): otherwise a
                // native/by-construction resolution (`arena_alloc`,
                // `fn_free`) or a loud link error for genuinely unknown
                // names must be preserved. In particular this never fires
                // in unaliased programs.
                let mut call_name_str = call_name_str;
                {
                    let bare_check = call_name_str.rsplit("::").next().unwrap_or(&call_name_str);
                    let bare_check = bare_check.rsplit("__").next().unwrap_or(bare_check);
                    let is_bare = !call_name_str.contains("::") && !call_name_str.contains("__");
                    let ambiguous = self.ctx.functions.iter().any(|f| {
                        let n = f.replace("::", "__");
                        n != call_name_str && n.ends_with(&format!("__{}", bare_check))
                    });
                    if is_bare
                        && ambiguous
                        && !self.ctx.functions.contains(&call_name_str)
                        && !self
                            .ctx
                            .extern_functions
                            .contains_key(call_name_str.as_str())
                        && !self.ctx.structs.contains_key(&call_name_str)
                    {
                        let suffix = format!("__{}", bare_check);
                        // Fewest qualifier segments first, namespaced (`::`)
                        // spellings before their plain `__` twins: every def
                        // is stored in both spellings, so an unordered pick
                        // would flip between processes (hash order).
                        let mut best: Option<(usize, bool, String)> = None;
                        for f in self.ctx.functions.iter() {
                            let n = f.replace("::", "__");
                            if n.ends_with(&suffix) {
                                let segs = n.split("__").count();
                                let plain = !f.contains("::");
                                let take = match &best {
                                    Some((bs, bp, _)) => (segs, plain) < (*bs, *bp),
                                    None => true,
                                };
                                if take {
                                    best = Some((segs, plain, f.clone()));
                                }
                            }
                        }
                        // Fire only for a namespaced GLOBAL (exactly two
                        // segments, e.g. `tensor::sum`): anything deeper is
                        // a method (`tensor::Pool__free`) that must keep its
                        // legacy resolution (natives like `fn_free`, link
                        // errors for unknowns). This also keeps the fallback
                        // silent in unaliased programs.
                        if let Some((2, false, hit)) = best {
                            call_name_str = hit;
                        }
                    }
                }
                let call_name = call_name_str.as_str();

                let initial_stack_offset = self.ctx.stack_offset;
                let word_size: i32 = match self.arch {
                    Architecture::ARM64 => 16,
                    _ => 8,
                };
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
                                self.ctx
                                    .variables
                                    .get(&format!("fn_param_interface:{}:{}", bare, param_idx))
                            })
                            .and_then(|vt| match vt {
                                VarType::Interface { interface_name, .. } => {
                                    Some(interface_name.clone())
                                }
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
                        arch::emit_fat_ptr_new(
                            &mut self.output,
                            self.arch,
                            &vtable_label,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    arch::emit_push_temp(&mut self.output, self.arch);
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
                    let is_flt_ret = self
                        .ctx
                        .variables
                        .contains_key(&format!("fn_ret_flt:{}", call_name))
                        || self
                            .ctx
                            .variables
                            .contains_key(&format!("fn_ret_flt:{}", extern_name))
                        || self
                            .ctx
                            .extern_functions
                            .get(call_name)
                            .or_else(|| self.ctx.extern_functions.get(extern_name))
                            .and_then(|info| info.return_type.as_deref())
                            .is_some_and(|rt| rt == "float" || rt == "f64" || rt == "f32");
                    if is_flt_ret {
                        match self.arch {
                            Architecture::X64 => {
                                self.output.push_str("    movq %xmm0, %rax\n");
                            }
                            Architecture::ARM64 => {
                                self.output.push_str("    fmov x0, d0\n");
                            }
                        }
                    } else {
                        let is_i32_ret = self
                            .ctx
                            .extern_functions
                            .get(call_name)
                            .or_else(|| self.ctx.extern_functions.get(extern_name))
                            .and_then(|info| info.return_type.as_deref())
                            .is_some_and(|rt| rt == "i32");
                        if is_i32_ret {
                            match self.arch {
                                Architecture::X64 => {
                                    self.output.push_str("    movslq %eax, %rax\n");
                                }
                                Architecture::ARM64 => {
                                    self.output.push_str("    sxtw x0, w0\n");
                                }
                            }
                        }
                    }
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
                // loop below stores with matching stride (same definition
                // resolution).
                let bare_init = name.rsplit("::").next().unwrap_or(name);
                let bare_init = bare_init.rsplit("__").next().unwrap_or(bare_init);
                let init_sdef = self
                    .ctx
                    .structs
                    .get(name)
                    .or_else(|| self.ctx.structs.get(bare_init))
                    .cloned();
                // itself (alya-lang/alya#62).
                let field_count = init_sdef
                    .as_ref()
                    .map_or_else(|| fields.len(), |s| s.fields.len());
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
                        // Mixed literal kinds for this field (e.g. `Box{value: 1}`
                        // and `Box{value: "s"}`): no single static marker
                        // serves every instance, so skip global markers and
                        // let reads fall back to runtime classification plus
                        // per-variable keys. Code emission below is unaffected.
                        if !struct_field_markers_mixed_vars(&self.ctx.variables, name, fname) {
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
                                self.ctx.variables.insert(
                                    format!("struct_field_flt:{}", fname),
                                    VarType::Float(0),
                                );
                            } else if is_arr {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}.{}", name, fname),
                                    VarType::Array(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}", fname),
                                    VarType::Array(0),
                                );
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
                } else {
                    for (fname, fval) in fields {
                        let is_str = is_string_expr(fval, &self.ctx.variables);
                        let is_flt = is_float_expr(fval, &self.ctx.variables);
                        let is_arr = is_array_expr(fval, &self.ctx.variables);
                        let is_map = is_map_expr(fval, &self.ctx.variables);
                        // Mixed literal kinds for this field (e.g. `Box{value: 1}`
                        // and `Box{value: "s"}`): no single static marker
                        // serves every instance, so skip global markers and
                        // let reads fall back to runtime classification plus
                        // per-variable keys. Code emission below is unaffected.
                        if !struct_field_markers_mixed_vars(&self.ctx.variables, name, fname) {
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
                                self.ctx.variables.insert(
                                    format!("struct_field_flt:{}", fname),
                                    VarType::Float(0),
                                );
                            } else if is_arr {
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}.{}", name, fname),
                                    VarType::Array(0),
                                );
                                self.ctx.variables.insert(
                                    format!("struct_field_arr:{}", fname),
                                    VarType::Array(0),
                                );
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
                }
                arch::emit_push_temp(&mut self.output, self.arch);
                let temp_offset: i32 = match self.arch {
                    Architecture::ARM64 => 16,
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
                                &Expr::Number(0)
                            };
                        let is_weak = sdef
                            .field_types
                            .get(i)
                            .and_then(|t| t.as_deref())
                            .is_some_and(|t| t.starts_with("weak ") || t == "weak");
                        self.generate_expression(arg_expr);
                        if !is_weak && self.is_heap_expression(arg_expr) {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        // B1: named stores outlive the wrapping ring buffer.
                        if string_store_needs_dup(arg_expr, &self.ctx.variables) {
                            arch::emit_str_store(
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
                        // B1: named stores outlive the wrapping ring buffer.
                        if string_store_needs_dup(fval, &self.ctx.variables) {
                            arch::emit_str_store(
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
                if field == "message" && is_string_expr(object, &self.ctx.variables) {
                    self.generate_expression(object);
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    movq %rax, %xmm0\n");
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    fmov d0, x0\n");
                        }
                    }
                    return;
                }
                if let Expr::Identifier(obj_name) = &**object {
                    if !self.ctx.variables.contains_key(obj_name) {
                        let cand_double = format!("{}__{}", obj_name, field);
                        let cand_colon = format!("{}::{}", obj_name, field);
                        if let Some((symbol, _)) = self
                            .ctx
                            .globals
                            .get(field)
                            .or_else(|| self.ctx.globals.get(&cand_double))
                            .or_else(|| self.ctx.globals.get(&cand_colon))
                        {
                            arch::emit_load_global(&mut self.output, self.arch, symbol, self.os);
                            return;
                        }
                    }
                }
                let field_idx = self
                    .resolve_struct_field_target(object, field)
                    .map(|(_, idx)| idx)
                    .unwrap_or(0);

                let is_weak = self.is_struct_field_weak(object, field);
                self.generate_expression(object);
                // Null-base trap (alya-lang/alya#74): same rationale as the
                // field-store trap in `generate_field_assign`. Plain
                // `FieldAccess` only (`?.` forms are separate variants
                // handled with short-circuit semantics elsewhere).
                arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                arch::emit_cond_jump(
                    &mut self.output,
                    self.arch,
                    BinaryOp::Equal,
                    false,
                    "alya_error_null_field",
                    false,
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
                    _ => 8,
                };
                self.ctx.stack_offset += temp_offset;

                for (i, elem) in elements.iter().enumerate() {
                    // Fresh-owned elements (literals, direct struct
                    // constructors, freshness-marked calls) move into the
                    // array: no retain is needed and no producer temp is
                    // left behind (alya-lang/alya#79). Other calls and
                    // borrowed values keep the retaining share; their
                    // producer temp cannot be proven separate (aliasing
                    // calls), so it stays a residual leak rather than
                    // risking a use-after-free.
                    let elem_moves = matches!(
                        elem,
                        Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. }
                    ) || match elem {
                        Expr::Call { name, .. } => {
                            self.ctx.structs.contains_key(name)
                                || crate::codegen::analysis::call_returns_fresh_value(
                                    name,
                                    &self.ctx.variables,
                                )
                        }
                        _ => false,
                    };
                    self.generate_expression(elem);
                    if self.is_heap_expression(elem) && !elem_moves {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    // B1: named stores outlive the wrapping ring buffer.
                    if string_store_needs_dup(elem, &self.ctx.variables) {
                        arch::emit_str_store(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    let elem_kind = value_kind_tag(elem, &self.ctx.variables);
                    arch::emit_array_set_imm(&mut self.output, self.arch, i, elem_kind);
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
                        // B1: named stores outlive the wrapping ring buffer.
                        if string_store_needs_dup(k, &self.ctx.variables) {
                            arch::emit_str_store(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        let k_offset = self.ctx.stack_offset;

                        self.generate_expression(v);
                        // B1: named stores outlive the wrapping ring buffer.
                        if string_store_needs_dup(v, &self.ctx.variables) {
                            arch::emit_str_store(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_allocate_var(
                            &mut self.output,
                            self.arch,
                            &mut self.ctx.stack_offset,
                        );
                        let v_offset = self.ctx.stack_offset;
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
                        let v_moves =
                            matches!(v, Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. })
                                || match v {
                                    Expr::Call { name, .. } => {
                                        self.ctx.structs.contains_key(name)
                                            || crate::codegen::analysis::call_returns_fresh_value(
                                                name,
                                                &self.ctx.variables,
                                            )
                                    }
                                    _ => false,
                                };
                        if self.store_value_needs_retain(v) && !v_moves {
                            arch::emit_rc_retain(
                                &mut self.output,
                                self.arch,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }
                        arch::emit_push_temp(&mut self.output, self.arch);

                        arch::emit_function_call(
                            &mut self.output,
                            self.arch,
                            "set",
                            3,
                            self.ctx.stack_offset,
                            self.os,
                        );
                        // Record the entry tag for literal values, mirroring
                        // IndexAssign (alya-lang/alya#39): `set` clears tags,
                        // so literal-built maps would otherwise read back
                        // unknown (raw bits in loops, classify fallback in
                        // `say`). Slots reload without re-evaluating, so any
                        // key shape is safe.
                        let kind = crate::codegen::analysis::value_kind_tag(v, &self.ctx.variables);
                        if kind != crate::codegen::kinds::KIND_UNKNOWN {
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
                            arch::emit_load_num(&mut self.output, self.arch, kind);
                            arch::emit_push_temp(&mut self.output, self.arch);
                            arch::emit_function_call(
                                &mut self.output,
                                self.arch,
                                "map_set_tag",
                                3,
                                self.ctx.stack_offset,
                                self.os,
                            );
                        }

                        arch::emit_pop_temp(&mut self.output, self.arch);
                        arch::emit_pop_temp(&mut self.output, self.arch);
                        self.ctx.stack_offset -= match self.arch {
                            Architecture::ARM64 => 32,
                            _ => 16,
                        };
                    }

                    arch::emit_pop_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset -= match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                }
            }
            Expr::Index { array, index } => {
                if let Some(sname) = self.get_expr_struct_name(array) {
                    let bare_sname = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_sname = bare_sname.rsplit("__").next().unwrap_or(bare_sname);
                    let cand1 = format!("{}__{}", sname, "operator[]");
                    let cand2 = format!("{}__{}", bare_sname, "operator[]");
                    let matched = if self.ctx.functions.contains(&cand1) {
                        Some(cand1)
                    } else if self.ctx.functions.contains(&cand2) {
                        Some(cand2)
                    } else {
                        self.ctx
                            .functions
                            .iter()
                            .find(|f| f.ends_with("__operator[]"))
                            .cloned()
                    };
                    if let Some(call_name) = matched {
                        self.generate_expression(&Expr::Call {
                            name: call_name,
                            args: vec![(**array).clone(), (**index).clone()],
                        });
                        return;
                    }
                }

                if is_map_expr(array, &self.ctx.variables)
                    || is_string_expr(index, &self.ctx.variables)
                    || matches!(**index, Expr::String(_))
                {
                    let default_val = Expr::Number(0);
                    let actual_args = [array.as_ref(), index.as_ref(), &default_val];
                    let initial_stack_offset = self.ctx.stack_offset;
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                    for (idx, arg) in actual_args.iter().enumerate() {
                        self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                        self.generate_expression(arg);
                        arch::emit_push_temp(&mut self.output, self.arch);
                    }
                    self.ctx.stack_offset = initial_stack_offset;
                    arch::emit_function_call(
                        &mut self.output,
                        self.arch,
                        "get",
                        3,
                        self.ctx.stack_offset,
                        self.os,
                    );
                } else if is_string_expr(array, &self.ctx.variables) {
                    if let Expr::Binary { left, op, right } = &**index {
                        if matches!(op, BinaryOp::Range | BinaryOp::RangeInclusive) {
                            let len_expr = match op {
                                BinaryOp::Range => Expr::Binary {
                                    left: right.clone(),
                                    op: BinaryOp::Subtract,
                                    right: left.clone(),
                                },
                                BinaryOp::RangeInclusive => Expr::Binary {
                                    left: Box::new(Expr::Binary {
                                        left: right.clone(),
                                        op: BinaryOp::Subtract,
                                        right: left.clone(),
                                    }),
                                    op: BinaryOp::Add,
                                    right: Box::new(Expr::Number(1)),
                                },
                                _ => unreachable!(),
                            };
                            let actual_args = [array.as_ref(), left.as_ref(), &len_expr];
                            let initial_stack_offset = self.ctx.stack_offset;
                            let word_size: i32 = match self.arch {
                                Architecture::ARM64 => 16,
                                _ => 8,
                            };
                            for (idx, arg) in actual_args.iter().enumerate() {
                                self.ctx.stack_offset =
                                    initial_stack_offset + (idx as i32 * word_size);
                                self.generate_expression(arg);
                                arch::emit_push_temp(&mut self.output, self.arch);
                            }
                            self.ctx.stack_offset = initial_stack_offset;
                            arch::emit_function_call(
                                &mut self.output,
                                self.arch,
                                "substring",
                                3,
                                self.ctx.stack_offset,
                                self.os,
                            );
                            return;
                        }
                    }

                    let actual_args = [array.as_ref(), index.as_ref()];
                    let initial_stack_offset = self.ctx.stack_offset;
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                    for (idx, arg) in actual_args.iter().enumerate() {
                        self.ctx.stack_offset = initial_stack_offset + (idx as i32 * word_size);
                        self.generate_expression(arg);
                        arch::emit_push_temp(&mut self.output, self.arch);
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
                if field == "message" && is_string_expr(object, &self.ctx.variables) {
                    self.generate_expression(object);
                    match self.arch {
                        Architecture::X64 => {
                            self.output.push_str("    movq %rax, %xmm0\n");
                        }
                        Architecture::ARM64 => {
                            self.output.push_str("    fmov d0, x0\n");
                        }
                    }
                    return;
                }
                let null_label = self.ctx.next_label();
                let end_label = self.ctx.next_label();

                if let Expr::Identifier(obj_name) = &**object {
                    if !self.ctx.variables.contains_key(obj_name) {
                        let cand_double = format!("{}__{}", obj_name, field);
                        let cand_colon = format!("{}::{}", obj_name, field);
                        if let Some((symbol, _)) = self
                            .ctx
                            .globals
                            .get(field)
                            .or_else(|| self.ctx.globals.get(&cand_double))
                            .or_else(|| self.ctx.globals.get(&cand_colon))
                        {
                            arch::emit_load_global(&mut self.output, self.arch, symbol, self.os);
                            return;
                        }
                    }
                }
                let field_idx = self
                    .resolve_struct_field_target(object, field)
                    .map(|(_, idx)| idx)
                    .unwrap_or(0);

                let is_weak = self.is_struct_field_weak(object, field);

                self.generate_expression(object);
                arch::emit_cmp_imm(&mut self.output, self.arch, 0);
                arch::emit_cond_jump(
                    &mut self.output,
                    self.arch,
                    BinaryOp::Equal,
                    false,
                    &null_label,
                    false,
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
                    false,
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
                        false,
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
                        false,
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
            Expr::Cast { expr, target } => {
                self.generate_expression(expr);
                let t = target.to_lowercase();
                if (t == "int" || t == "i64" || t == "rune")
                    && is_string_expr(expr, &self.ctx.variables)
                {
                    match self.arch {
                        Architecture::X64 => {
                            let end_lbl = self.ctx.next_label();
                            let multi_lbl = self.ctx.next_label();
                            let chk3_lbl = self.ctx.next_label();
                            let chk4_lbl = self.ctx.next_label();
                            self.output.push_str("    movzbq (%rax), %rcx\n");
                            self.output.push_str("    cmp $0x80, %rcx\n");
                            self.output.push_str(&format!("    jae {}\n", multi_lbl));
                            self.output.push_str("    mov %rcx, %rax\n");
                            self.output.push_str(&format!("    jmp {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", multi_lbl));
                            self.output.push_str("    mov %rcx, %rdx\n");
                            self.output.push_str("    and $0xE0, %rdx\n");
                            self.output.push_str("    cmp $0xC0, %rdx\n");
                            self.output.push_str(&format!("    jne {}\n", chk3_lbl));
                            self.output.push_str("    and $0x1F, %rcx\n");
                            self.output.push_str("    shl $6, %rcx\n");
                            self.output.push_str("    movzbq 1(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    mov %rcx, %rax\n");
                            self.output.push_str(&format!("    jmp {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", chk3_lbl));
                            self.output.push_str("    mov %rcx, %rdx\n");
                            self.output.push_str("    and $0xF0, %rdx\n");
                            self.output.push_str("    cmp $0xE0, %rdx\n");
                            self.output.push_str(&format!("    jne {}\n", chk4_lbl));
                            self.output.push_str("    and $0x0F, %rcx\n");
                            self.output.push_str("    shl $12, %rcx\n");
                            self.output.push_str("    movzbq 1(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    shl $6, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    movzbq 2(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    mov %rcx, %rax\n");
                            self.output.push_str(&format!("    jmp {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", chk4_lbl));
                            self.output.push_str("    and $0x07, %rcx\n");
                            self.output.push_str("    shl $18, %rcx\n");
                            self.output.push_str("    movzbq 1(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    shl $12, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    movzbq 2(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    shl $6, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    movzbq 3(%rax), %rdx\n");
                            self.output.push_str("    and $0x3F, %rdx\n");
                            self.output.push_str("    or %rdx, %rcx\n");
                            self.output.push_str("    mov %rcx, %rax\n");
                            self.output.push_str(&format!("{}:\n", end_lbl));
                        }
                        Architecture::ARM64 => {
                            let end_lbl = self.ctx.next_label();
                            let multi_lbl = self.ctx.next_label();
                            let chk3_lbl = self.ctx.next_label();
                            let chk4_lbl = self.ctx.next_label();
                            self.output.push_str("    ldrb w1, [x0]\n");
                            self.output.push_str("    cmp w1, #0x80\n");
                            self.output.push_str(&format!("    b.hs {}\n", multi_lbl));
                            self.output.push_str("    mov x0, x1\n");
                            self.output.push_str(&format!("    b {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", multi_lbl));
                            self.output.push_str("    and w2, w1, #0xE0\n");
                            self.output.push_str("    cmp w2, #0xC0\n");
                            self.output.push_str(&format!("    b.ne {}\n", chk3_lbl));
                            self.output.push_str("    and w1, w1, #0x1F\n");
                            self.output.push_str("    lsl w1, w1, #6\n");
                            self.output.push_str("    ldrb w2, [x0, #1]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    orr w0, w1, w2\n");
                            self.output.push_str(&format!("    b {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", chk3_lbl));
                            self.output.push_str("    and w2, w1, #0xF0\n");
                            self.output.push_str("    cmp w2, #0xE0\n");
                            self.output.push_str(&format!("    b.ne {}\n", chk4_lbl));
                            self.output.push_str("    and w1, w1, #0x0F\n");
                            self.output.push_str("    lsl w1, w1, #12\n");
                            self.output.push_str("    ldrb w2, [x0, #1]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    lsl w2, w2, #6\n");
                            self.output.push_str("    orr w1, w1, w2\n");
                            self.output.push_str("    ldrb w2, [x0, #2]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    orr w0, w1, w2\n");
                            self.output.push_str(&format!("    b {}\n", end_lbl));
                            self.output.push_str(&format!("{}:\n", chk4_lbl));
                            self.output.push_str("    and w1, w1, #0x07\n");
                            self.output.push_str("    lsl w1, w1, #18\n");
                            self.output.push_str("    ldrb w2, [x0, #1]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    lsl w2, w2, #12\n");
                            self.output.push_str("    orr w1, w1, w2\n");
                            self.output.push_str("    ldrb w2, [x0, #2]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    lsl w2, w2, #6\n");
                            self.output.push_str("    orr w1, w1, w2\n");
                            self.output.push_str("    ldrb w2, [x0, #3]\n");
                            self.output.push_str("    and w2, w2, #0x3F\n");
                            self.output.push_str("    orr w0, w1, w2\n");
                            self.output.push_str(&format!("{}:\n", end_lbl));
                        }
                    }
                } else if t == "float" || t == "f64" {
                    // int -> float (Chapter 02 §1.4). Float and string operands
                    // pass through (strings: use float("...") conversion).
                    // Kind-carrying reads already holding float bits skip
                    // the conversion (else cvt mangles them).
                    if !is_float_expr(expr, &self.ctx.variables)
                        && !is_string_expr(expr, &self.ctx.variables)
                    {
                        let already_float =
                            matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                                && matches!(
                                    &**expr,
                                    Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                                )
                                && is_tag_carrying_read(expr, &self.ctx.variables);
                        if already_float {
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
                } else if t == "int"
                    || t == "i64"
                    || t == "isize"
                    || t == "rune"
                    || t == "uint"
                    || t == "u64"
                    || t == "usize"
                    || t == "ptr"
                {
                    // float -> int truncation; same-width integers are no-ops.
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                } else if t == "u8" || t == "byte" {
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                    match self.arch {
                        Architecture::X64 => self.output.push_str("    and $0xFF, %rax\n"),
                        Architecture::ARM64 => self.output.push_str("    and x0, x0, #0xff\n"),
                    }
                } else if t == "u16" {
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                    match self.arch {
                        Architecture::X64 => self.output.push_str("    and $0xFFFF, %rax\n"),
                        Architecture::ARM64 => self.output.push_str("    and x0, x0, #0xffff\n"),
                    }
                } else if t == "i8" {
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                    match self.arch {
                        Architecture::X64 => self.output.push_str("    movsbq %al, %rax\n"),
                        Architecture::ARM64 => self.output.push_str("    sxtb x0, w0\n"),
                    }
                } else if t == "i16" {
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                    match self.arch {
                        Architecture::X64 => self.output.push_str("    movswq %ax, %rax\n"),
                        Architecture::ARM64 => self.output.push_str("    sxth x0, w0\n"),
                    }
                } else if t == "i32" || t == "u32" {
                    if is_float_expr(expr, &self.ctx.variables) {
                        arch::emit_float_to_int(&mut self.output, self.arch);
                    }
                    // 32-bit moves zero-extend on x64/ARM64.
                    match self.arch {
                        Architecture::X64 => self.output.push_str("    mov %eax, %eax\n"),
                        Architecture::ARM64 => self.output.push_str("    mov w0, w0\n"),
                    }
                }
            }
        }
    }

    /// Exact static `typeof` vocabulary for literals (alya-lang/alya#71).
    /// Everything else dispatches at runtime via `emit_typeof_cascade`.
    fn typeof_literal_kind(expr: &Expr) -> Option<&'static str> {
        match expr {
            Expr::Number(_) | Expr::Null => Some("int"),
            Expr::Float(_) => Some("float"),
            Expr::String(_) | Expr::InterpolatedString(_) => Some("string"),
            Expr::Array(_) => Some("array"),
            Expr::Map(_) => Some("map"),
            _ => None,
        }
    }

    /// Runtime `typeof` dispatch (alya-lang/alya#71): a cascade of `is`
    /// checks over one operand, ending in the historical `"int"`
    /// fallback (ints, nulls, bools and anything else unrecognized all
    /// report `"int"`, matching the literal vocabulary). Reuses the
    /// verified dynamic `is` paths, so every branch discriminates at
    /// runtime. Repeatable operands re-evaluate per branch (observably
    /// identical); anything else evaluates once into a hidden
    /// single-element cell (`__typeof_cell` frame slot) and branches
    /// read element 0 — index reads carry no may-markings, so every
    /// check dispatches at runtime. No retain/release delta: checks
    /// never consume their operand, the same ownership profile as `is`
    /// on a call today.
    fn emit_typeof_cascade(&mut self, operand: &Expr) {
        let branch_operand: Expr;
        let mut prev_binding = None;
        if typeof_operand_is_repeatable(operand) {
            branch_operand = operand.clone();
        } else {
            arch::emit_allocate_var(&mut self.output, self.arch, &mut self.ctx.stack_offset);
            let slot = self.ctx.stack_offset;
            self.generate_expression(&Expr::Array(vec![operand.clone()]));
            arch::emit_store_var(&mut self.output, self.arch, slot, self.ctx.stack_offset);
            prev_binding = self
                .ctx
                .variables
                .insert("__typeof_cell".to_string(), VarType::Array(slot));
            branch_operand = Expr::Index {
                array: Box::new(Expr::Identifier("__typeof_cell".to_string())),
                index: Box::new(Expr::Number(0)),
            };
        }
        let end_label = self.ctx.next_label();
        let kinds: &[(&str, &str)] = &[
            ("array", "array"),
            ("map", "map"),
            ("string", "string"),
            ("float", "float"),
        ];
        for (target, kind_name) in kinds {
            let next_label = self.ctx.next_label();
            self.generate_condition_jump_if_false(
                &Expr::TypeCheck {
                    expr: Box::new(branch_operand.clone()),
                    target: target.to_string(),
                    negated: false,
                },
                &next_label,
            );
            self.generate_expression(&Expr::String(kind_name.to_string()));
            arch::emit_jump(&mut self.output, self.arch, &end_label);
            self.output.push_str(&format!("{}:\n", next_label));
        }
        self.generate_expression(&Expr::String("int".to_string()));
        self.output.push_str(&format!("{}:\n", end_label));
        if prev_binding.is_some() || self.ctx.variables.contains_key("__typeof_cell") {
            match prev_binding {
                Some(prev) => {
                    self.ctx.variables.insert("__typeof_cell".to_string(), prev);
                }
                None => {
                    self.ctx.variables.remove("__typeof_cell");
                }
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
                // Variable-key map reads and (Phase 1, #39) plain array
                // reads carry the kind tag alongside the value
                // (x64: %edx, arm64: w1). Tag 3 = string.
                if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                    && matches!(
                        expr,
                        Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                    )
                    && is_tag_carrying_read(expr, &self.ctx.variables)
                {
                    self.generate_expression(expr);
                    let l_true = self.ctx.next_label();
                    let l_end = self.ctx.next_label();
                    let l_str = self.ctx.next_label();
                    let l_no = self.ctx.next_label();
                    if matches!(self.arch, Architecture::X64) {
                        // %rax = value (kept for tag==0 fallback below),
                        // %edx = tag (0 unknown, 3 string).
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_STRING));
                        self.output.push_str(&format!("    je {}\n", l_true));
                        // Unknown tag: fall back to pointer-range string test.
                        // Definite non-string tags (1, 2, 4, 5, 6) are
                        // boolean false, not the leftover value.
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_UNKNOWN));
                        self.output.push_str(&format!("    jne {}\n", l_no));
                        // Reuse the value in %rax for a light string check:
                        // rodata or str_buf range => string.
                        self.output.push_str("    cmp $65536, %rax\n");
                        self.output.push_str(&format!("    jb {}\n", l_no));
                        self.output
                            .push_str("    lea alya_rodata_start(%rip), %rcx\n");
                        self.output.push_str("    cmp %rcx, %rax\n");
                        self.output.push_str(&format!("    jb {}\n", l_no));
                        self.output
                            .push_str("    lea alya_rodata_end(%rip), %rcx\n");
                        self.output.push_str("    cmp %rcx, %rax\n");
                        self.output.push_str(&format!("    jb {}\n", l_str));
                        self.output.push_str("    lea alya_str_buf(%rip), %rcx\n");
                        self.output.push_str("    cmp %rcx, %rax\n");
                        self.output.push_str(&format!("    jb {}\n", l_no));
                        self.output.push_str("    lea 67108864(%rcx), %rcx\n");
                        self.output.push_str("    cmp %rcx, %rax\n");
                        self.output.push_str(&format!("    jb {}\n", l_str));
                        self.output.push_str(&format!("{}:\n", l_no));
                        if negated {
                            self.output.push_str("    movq $1, %rax\n");
                        } else {
                            self.output.push_str("    xor %eax, %eax\n");
                        }
                        self.output.push_str(&format!("    jmp {}\n", l_end));
                        self.output.push_str(&format!("{}:\n", l_str));
                        self.output.push_str(&format!("{}:\n", l_true));
                        if negated {
                            self.output.push_str("    xor %eax, %eax\n");
                        } else {
                            self.output.push_str("    movq $1, %rax\n");
                        }
                        self.output.push_str(&format!("{}:\n", l_end));
                    } else {
                        // arm64: x0 = value, w1 = tag.
                        let l_str = self.ctx.next_label();
                        let l_no = self.ctx.next_label();
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_STRING));
                        self.output.push_str(&format!("    b.eq {}\n", l_true));
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_UNKNOWN));
                        self.output.push_str(&format!("    b.ne {}\n", l_no));
                        self.output.push_str("    movz x2, #1, lsl #16\n");
                        self.output.push_str("    cmp x0, x2\n");
                        self.output.push_str(&format!("    b.lo {}\n", l_no));
                        crate::codegen::arch::arm64::emit_adrp_add(
                            &mut self.output,
                            "x2",
                            "alya_rodata_start",
                            self.os,
                        );
                        self.output.push_str("    cmp x0, x2\n");
                        self.output.push_str(&format!("    b.lo {}\n", l_no));
                        crate::codegen::arch::arm64::emit_adrp_add(
                            &mut self.output,
                            "x3",
                            "alya_rodata_end",
                            self.os,
                        );
                        self.output.push_str("    cmp x0, x3\n");
                        self.output.push_str(&format!("    b.lo {}\n", l_str));
                        crate::codegen::arch::arm64::emit_adrp_add(
                            &mut self.output,
                            "x2",
                            "alya_str_buf",
                            self.os,
                        );
                        self.output.push_str("    cmp x0, x2\n");
                        self.output.push_str(&format!("    b.lo {}\n", l_no));
                        self.output.push_str("    movz x3, #1024, lsl #16\n");
                        self.output.push_str("    add x3, x2, x3\n");
                        self.output.push_str("    cmp x0, x3\n");
                        self.output.push_str(&format!("    b.lo {}\n", l_str));
                        self.output.push_str(&format!("{}:\n", l_no));
                        if negated {
                            self.output.push_str("    mov x0, #1\n");
                        } else {
                            self.output.push_str("    mov x0, #0\n");
                        }
                        self.output.push_str(&format!("    b {}\n", l_end));
                        self.output.push_str(&format!("{}:\n", l_str));
                        self.output.push_str(&format!("{}:\n", l_true));
                        if negated {
                            self.output.push_str("    mov x0, #0\n");
                        } else {
                            self.output.push_str("    mov x0, #1\n");
                        }
                        self.output.push_str(&format!("{}:\n", l_end));
                    }
                    return;
                }
                let is_str = is_string_fold_true(expr, &self.ctx.variables);
                if is_str {
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 0 } else { 1 });
                } else if is_float_expr(expr, &self.ctx.variables)
                    || is_array_fold_true(expr, &self.ctx.variables)
                    || is_map_fold_true(expr, &self.ctx.variables)
                    || is_null_expr(expr, &self.ctx.variables)
                    || matches!(expr, Expr::Number(_) | Expr::Float(_))
                {
                    // Provably non-string: fold to a constant. (Number/Float
                    // cover literals; vars and calls stay dynamic below even
                    // when currently number-typed, since reassignment or an
                    // unannotated signature may carry a string at runtime.
                    // The pointer-range test below classifies those
                    // correctly, including the float-bits land in int bucket
                    // rule with its documented rodata/str-buf edge.)
                    arch::emit_load_num(&mut self.output, self.arch, if negated { 1 } else { 0 });
                } else {
                    // Truly unknown (unannotated params, unmarked vars and
                    // calls, struct values): test pointer ranges at runtime.
                    // Memory-safe: no dereference, only address compares.
                    self.generate_expression(expr);
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.emit_runtime_classify(self.os);
                    match self.arch {
                        Architecture::X64 => {
                            arch::emit_cmp_imm(&mut self.output, self.arch, KIND_STRING);
                            self.output.push_str(if negated {
                                "    setne %al\n"
                            } else {
                                "    sete %al\n"
                            });
                            self.output.push_str("    movzbq %al, %rax\n");
                        }
                        Architecture::ARM64 => {
                            self.output
                                .push_str(&format!("    cmp x0, #{}\n", KIND_STRING));
                            self.output.push_str(if negated {
                                "    cset x0, ne\n"
                            } else {
                                "    cset x0, eq\n"
                            });
                        }
                    }
                    // Discard the saved value without touching the boolean
                    // now in the return register (a pop would overwrite it).
                    let discard = self.temp_offset();
                    arch::emit_stack_restore(&mut self.output, self.arch, discard);
                }
            }
            "array" | "list" => {
                // alya-lang/alya#70: fold to true only on exact evidence.
                // The legacy `is_array_expr` trusts the may-marking, which
                // also covers merely-possible arrays.
                let is_arr = is_array_fold_true(expr, &self.ctx.variables);
                let is_def_non = is_string_fold_true(expr, &self.ctx.variables)
                    || is_map_fold_true(expr, &self.ctx.variables)
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
                // alya-lang/alya#70: same exactness contract as above.
                let is_map = is_map_fold_true(expr, &self.ctx.variables);
                let is_def_non = is_string_fold_true(expr, &self.ctx.variables)
                    || is_array_fold_true(expr, &self.ctx.variables)
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
                // Index carries kind tag alongside the value
                // (x64: %edx, arm64: w1; 2 = float), via map routing or
                // (Phase 1, #39) plain array reads.
                if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                    && matches!(
                        expr,
                        Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                    )
                    && is_tag_carrying_read(expr, &self.ctx.variables)
                {
                    self.generate_expression(expr);
                    if matches!(self.arch, Architecture::X64) {
                        if negated {
                            self.output
                                .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                            self.output.push_str("    setne %al\n");
                        } else {
                            self.output
                                .push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
                            self.output.push_str("    sete %al\n");
                        }
                        self.output.push_str("    movzbq %al, %rax\n");
                    } else if negated {
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                        self.output.push_str("    cset x0, ne\n");
                    } else {
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
                        self.output.push_str("    cset x0, eq\n");
                    }
                    return;
                }
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
                // Index tag 1=int (0 unknown defaults to int).
                // x64: %edx, arm64: w1; via map routing or (Phase 1, #39)
                // plain array reads.
                if matches!(self.arch, Architecture::X64 | Architecture::ARM64)
                    && matches!(
                        expr,
                        Expr::Index { .. } | Expr::Ternary { .. } | Expr::Call { .. }
                    )
                    && is_tag_carrying_read(expr, &self.ctx.variables)
                {
                    self.generate_expression(expr);
                    let l_true = self.ctx.next_label();
                    let l_end = self.ctx.next_label();
                    if matches!(self.arch, Architecture::X64) {
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_INT));
                        self.output.push_str(&format!("    je {}\n", l_true));
                        self.output
                            .push_str(&format!("    cmpl ${}, %edx\n", KIND_UNKNOWN));
                        self.output.push_str(&format!("    je {}\n", l_true));
                        if negated {
                            self.output.push_str("    movq $1, %rax\n");
                        } else {
                            self.output.push_str("    xor %eax, %eax\n");
                        }
                        self.output.push_str(&format!("    jmp {}\n", l_end));
                        self.output.push_str(&format!("{}:\n", l_true));
                        if negated {
                            self.output.push_str("    xor %eax, %eax\n");
                        } else {
                            self.output.push_str("    movq $1, %rax\n");
                        }
                        self.output.push_str(&format!("{}:\n", l_end));
                    } else {
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_INT));
                        self.output.push_str(&format!("    b.eq {}\n", l_true));
                        self.output
                            .push_str(&format!("    cmp w1, #{}\n", KIND_UNKNOWN));
                        self.output.push_str(&format!("    b.eq {}\n", l_true));
                        if negated {
                            self.output.push_str("    mov x0, #1\n");
                        } else {
                            self.output.push_str("    mov x0, #0\n");
                        }
                        self.output.push_str(&format!("    b {}\n", l_end));
                        self.output.push_str(&format!("{}:\n", l_true));
                        if negated {
                            self.output.push_str("    mov x0, #0\n");
                        } else {
                            self.output.push_str("    mov x0, #1\n");
                        }
                        self.output.push_str(&format!("{}:\n", l_end));
                    }
                    return;
                }
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
                let bare_target = target.rsplit("::").next().unwrap_or(target);
                let bare_target = bare_target.rsplit("__").next().unwrap_or(bare_target);

                let is_known_target_iface = self.ctx.interfaces.contains_key(target)
                    || self.ctx.interfaces.contains_key(bare_target)
                    || self.ctx.interfaces.values().any(|i| {
                        let bare = i.name.rsplit("::").next().unwrap_or(&i.name);
                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                        bare == bare_target
                    });
                let is_known_target_struct = self.ctx.structs.contains_key(target)
                    || self.ctx.structs.contains_key(bare_target)
                    || self.ctx.structs.values().any(|s| {
                        let bare = s.name.rsplit("::").next().unwrap_or(&s.name);
                        let bare = bare.rsplit("__").next().unwrap_or(bare);
                        bare == bare_target
                    });

                if is_known_target_iface {
                    if let Some(sname) = known_struct {
                        let bare_s = sname.rsplit("::").next().unwrap_or(&sname);
                        let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                        let satisfies = self
                            .ctx
                            .vtables
                            .contains_key(&(bare_s.to_string(), bare_target.to_string()))
                            || self
                                .ctx
                                .vtables
                                .contains_key(&(sname.clone(), target.to_string()))
                            || self.ctx.vtables.keys().any(|(s, i)| {
                                let bs = s.rsplit("::").next().unwrap_or(s);
                                let bs = bs.rsplit("__").next().unwrap_or(bs);
                                let bi = i.rsplit("::").next().unwrap_or(i);
                                let bi = bi.rsplit("__").next().unwrap_or(bi);
                                bs == bare_s && bi == bare_target
                            });
                        let res = if satisfies {
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
                        arch::emit_load_num(&mut self.output, self.arch, res);
                    } else {
                        let is_literal_non = matches!(
                            expr,
                            Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null
                        );
                        if is_literal_non {
                            arch::emit_load_num(
                                &mut self.output,
                                self.arch,
                                if negated { 1 } else { 0 },
                            );
                        } else {
                            self.emit_interface_type_check(expr, target, negated);
                        }
                    }
                } else if let Some(sname) = known_struct {
                    let matches = sname == target || sname.ends_with(&format!("::{}", target));
                    let res = if matches {
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
                    arch::emit_load_num(&mut self.output, self.arch, res);
                } else if is_known_target_struct {
                    let is_literal_non = matches!(
                        expr,
                        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null
                    );
                    if is_literal_non {
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
                self.output.push_str("    movl -16(%rax), %edx\n");
                self.output.push_str("    cmpl $0x5A110003, %edx\n");
                self.output
                    .push_str(&format!("    je {}\n", concrete_label));
                self.output.push_str("    cmpl $0x5A110004, %edx\n");
                self.output.push_str(&format!("    jne {}\n", false_label));
                // Fat pointer: load vtable at 8(%rax), then load descriptor from 0(%r11)
                self.output.push_str("    movq 8(%rax), %r11\n");
                self.output.push_str("    test %r11, %r11\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    movq (%r11), %r11\n");
                self.output
                    .push_str(&format!("    jmp {}\n", check_desc_label));
                // Concrete struct: load descriptor from 0(%rax)
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    movq (%rax), %r11\n");
                // Check descriptor against target descriptor
                self.output.push_str(&format!("{}:\n", check_desc_label));
                self.output
                    .push_str(&format!("    lea {}(%rip), %rdx\n", desc_label));
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
            Architecture::ARM64 => {
                self.output.push_str("    cbz x0, ");
                self.output.push_str(&format!("{}\n", false_label));
                self.output.push_str("    tst x0, #7\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str("    cmp x0, #65536\n");
                self.output.push_str(&format!("    b.ls {}\n", false_label));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output
                    .push_str(&format!("    cbnz x1, {}\n", false_label));
                self.output.push_str("    ldur x1, [x0, #-16]\n");
                self.output.push_str("    movz x2, #0x0003\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output
                    .push_str(&format!("    b.eq {}\n", concrete_label));
                self.output.push_str("    movz x2, #0x0004\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                // Fat pointer
                self.output.push_str("    ldr x2, [x0, #8]\n");
                self.output
                    .push_str(&format!("    cbz x2, {}\n", false_label));
                self.output.push_str("    ldr x2, [x2]\n");
                self.output
                    .push_str(&format!("    b {}\n", check_desc_label));
                // Concrete
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    ldr x2, [x0]\n");
                // Check desc
                self.output.push_str(&format!("{}:\n", check_desc_label));
                crate::codegen::arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x3",
                    &desc_label,
                    self.os,
                );
                self.output.push_str("    cmp x2, x3\n");
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

    fn emit_interface_type_check(&mut self, expr: &Expr, target: &str, negated: bool) {
        let bare_target = target.rsplit("::").next().unwrap_or(target);
        let bare_target = bare_target.rsplit("__").next().unwrap_or(bare_target);

        let matching_structs: Vec<String> = self
            .ctx
            .vtables
            .keys()
            .filter_map(|(s, i)| {
                let bi = i.rsplit("::").next().unwrap_or(i);
                let bi = bi.rsplit("__").next().unwrap_or(bi);
                if bi == bare_target {
                    let bs = s.rsplit("::").next().unwrap_or(s);
                    let bs = bs.rsplit("__").next().unwrap_or(bs);
                    Some(bs.to_string())
                } else {
                    None
                }
            })
            .collect();

        eprintln!(
            "IN emit_interface_type_check: target={}, matching_structs={:?}",
            target, matching_structs
        );

        let mut unique_structs = Vec::new();
        for s in matching_structs {
            if !unique_structs.contains(&s) {
                unique_structs.push(s);
            }
        }

        if unique_structs.is_empty() {
            arch::emit_load_num(&mut self.output, self.arch, if negated { 1 } else { 0 });
            return;
        }

        self.generate_expression(expr);
        let false_label = self.ctx.next_label();
        let true_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        let concrete_label = self.ctx.next_label();
        let check_desc_label = self.ctx.next_label();

        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    test %rax, %rax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    test $7, %rax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movl -16(%rax), %edx\n");
                self.output.push_str("    cmpl $0x5A110003, %edx\n");
                self.output
                    .push_str(&format!("    je {}\n", concrete_label));
                self.output.push_str("    cmpl $0x5A110004, %edx\n");
                self.output.push_str(&format!("    jne {}\n", false_label));
                // Fat pointer: load vtable at 8(%rax), then load descriptor from 0(%r11)
                self.output.push_str("    movq 8(%rax), %r11\n");
                self.output.push_str("    test %r11, %r11\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                self.output.push_str("    movq (%r11), %r11\n");
                self.output
                    .push_str(&format!("    jmp {}\n", check_desc_label));
                // Concrete struct: load descriptor from 0(%rax)
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    movq (%rax), %r11\n");
                // Check descriptor against target descriptor
                self.output.push_str(&format!("{}:\n", check_desc_label));
                for s in &unique_structs {
                    let desc_label = format!("alya_struct_desc_{}", s);
                    self.output
                        .push_str(&format!("    lea {}(%rip), %rdx\n", desc_label));
                    self.output.push_str("    cmp %rdx, %r11\n");
                    self.output.push_str(&format!("    je {}\n", true_label));
                }
                self.output.push_str(&format!("    jmp {}\n", false_label));
                self.output.push_str(&format!("{}:\n", true_label));
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
            Architecture::ARM64 => {
                self.output.push_str("    cbz x0, ");
                self.output.push_str(&format!("{}\n", false_label));
                self.output.push_str("    tst x0, #7\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str("    cmp x0, #65536\n");
                self.output.push_str(&format!("    b.ls {}\n", false_label));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output
                    .push_str(&format!("    cbnz x1, {}\n", false_label));
                self.output.push_str("    ldur x1, [x0, #-16]\n");
                self.output.push_str("    movz x2, #0x0003\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output
                    .push_str(&format!("    b.eq {}\n", concrete_label));
                self.output.push_str("    movz x2, #0x0004\n");
                self.output.push_str("    movk x2, #0x5A11, lsl #16\n");
                self.output.push_str("    cmp x1, x2\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                // Fat pointer
                self.output.push_str("    ldr x2, [x0, #8]\n");
                self.output
                    .push_str(&format!("    cbz x2, {}\n", false_label));
                self.output.push_str("    ldr x2, [x2]\n");
                self.output
                    .push_str(&format!("    b {}\n", check_desc_label));
                // Concrete
                self.output.push_str(&format!("{}:\n", concrete_label));
                self.output.push_str("    ldr x2, [x0]\n");
                // Check desc
                self.output.push_str(&format!("{}:\n", check_desc_label));
                for s in &unique_structs {
                    let desc_label = format!("alya_struct_desc_{}", s);
                    crate::codegen::arch::arm64::emit_adrp_add(
                        &mut self.output,
                        "x3",
                        &desc_label,
                        self.os,
                    );
                    self.output.push_str("    cmp x2, x3\n");
                    self.output.push_str(&format!("    b.eq {}\n", true_label));
                }
                self.output.push_str(&format!("    b {}\n", false_label));
                self.output.push_str(&format!("{}:\n", true_label));
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

    fn emit_runtime_tag_check(&mut self, expr: &Expr, tag: u64, negated: bool) {
        self.generate_expression(expr);
        let false_label = self.ctx.next_label();
        let end_label = self.ctx.next_label();
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    test %rax, %rax\n");
                self.output.push_str(&format!("    jz {}\n", false_label));
                // High-bits guard (mirrors the arm64 `lsr #47` below):
                // user heap addresses are below 2^47, but raw float
                // bits and negative ints are not. Dereferencing those
                // reads garbage or segfaults (alya-lang/alya#71);
                // skipping the header read reports a safe false.
                self.output.push_str("    mov %rax, %rdx\n");
                self.output.push_str("    shr $47, %rdx\n");
                self.output.push_str("    test %rdx, %rdx\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    test $7, %rax\n");
                self.output.push_str(&format!("    jnz {}\n", false_label));
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", false_label));
                self.output.push_str("    movl -16(%rax), %edx\n");
                self.output
                    .push_str(&format!("    cmpl $0x{:X}, %edx\n", tag as u32));
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
            Architecture::ARM64 => {
                self.output.push_str("    cbz x0, ");
                self.output.push_str(&format!("{}\n", false_label));
                self.output.push_str("    tst x0, #7\n");
                self.output.push_str(&format!("    b.ne {}\n", false_label));
                self.output.push_str("    cmp x0, #65536\n");
                self.output.push_str(&format!("    b.ls {}\n", false_label));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output
                    .push_str(&format!("    cbnz x1, {}\n", false_label));
                self.output.push_str("    ldur w1, [x0, #-16]\n");
                let tag_lo = tag as u32;
                self.output
                    .push_str(&format!("    movz w2, #{}\n", tag_lo & 0xFFFF));
                self.output
                    .push_str(&format!("    movk w2, #{}, lsl #16\n", tag_lo >> 16));
                self.output.push_str("    cmp w1, w2\n");
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

    /// Classifies a runtime value for dynamic dispatch: rax/eax/x0 holds
    /// the value on entry and the tag on exit (1 = int, 3 = string).
    /// Only consulted for statically-unknown values; every rule is chosen
    /// to be memory-safe (no unvalidated dereference): small/negative
    /// integers and everything outside the known string regions
    /// conservatively report int, matching historical behavior. Integer 0
    /// reports int (it shares a representation with null, and `say 0`
    /// must keep printing `0`). Floats are intentionally NOT detected
    /// (their bits are ambiguous with ints/pointers without tags) and
    /// land in the int bucket.
    pub(crate) fn emit_runtime_classify(&mut self, os: OperatingSystem) {
        let l_str = self.ctx.next_label();
        let l_int = self.ctx.next_label();
        let l_not_ro = self.ctx.next_label();
        let l_stable = self.ctx.next_label();
        let l_end = self.ctx.next_label();
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    cmp $65536, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_int));
                self.output.push_str("    mov $0x00007fffffffffff, %rdx\n");
                self.output.push_str("    cmp %rdx, %rax\n");
                self.output.push_str(&format!("    ja {}\n", l_int));
                self.output
                    .push_str("    lea alya_rodata_start(%rip), %rdx\n");
                self.output.push_str("    cmp %rdx, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_not_ro));
                self.output
                    .push_str("    lea alya_rodata_end(%rip), %rcx\n");
                self.output.push_str("    cmp %rcx, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_str));
                self.output.push_str(&format!("{}:\n", l_not_ro));
                self.output.push_str("    lea alya_str_buf(%rip), %rdx\n");
                self.output.push_str("    cmp %rdx, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_stable));
                self.output.push_str("    lea 67108864(%rdx), %rcx\n");
                self.output.push_str("    cmp %rcx, %rax\n");
                self.output.push_str(&format!("    jae {}\n", l_stable));
                self.output.push_str("    cmpb $0, (%rax)\n");
                self.output.push_str(&format!("    je {}\n", l_int));
                self.output.push_str(&format!("    jmp {}\n", l_str));
                // Stable store region (B1): immortal strings live here
                // whole-program — same string treatment as str_buf above.
                self.output.push_str(&format!("{}:\n", l_stable));
                self.output
                    .push_str("    lea alya_str_stable(%rip), %rdx\n");
                self.output.push_str("    cmp %rdx, %rax\n");
                self.output.push_str(&format!("    jb {}\n", l_int));
                self.output.push_str("    lea 67108864(%rdx), %rcx\n");
                self.output.push_str("    cmp %rcx, %rax\n");
                self.output.push_str(&format!("    jae {}\n", l_int));
                self.output.push_str("    cmpb $0, (%rax)\n");
                self.output.push_str(&format!("    je {}\n", l_int));
                self.output.push_str(&format!("    jmp {}\n", l_str));
                self.output.push_str(&format!("{}:\n", l_int));
                self.output
                    .push_str(&format!("    movq ${}, %rax\n", KIND_INT));
                self.output.push_str(&format!("    jmp {}\n", l_end));
                self.output.push_str(&format!("{}:\n", l_str));
                self.output
                    .push_str(&format!("    movq ${}, %rax\n", KIND_STRING));
                self.output.push_str(&format!("    jmp {}\n", l_end));
                self.output.push_str(&format!("{}:\n", l_end));
            }
            Architecture::ARM64 => {
                self.output.push_str("    movz x1, #1, lsl #16\n");
                self.output.push_str("    cmp x0, x1\n");
                self.output.push_str(&format!("    b.ls {}\n", l_int));
                self.output.push_str("    lsr x1, x0, #47\n");
                self.output.push_str(&format!("    cbnz x1, {}\n", l_int));
                crate::codegen::arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x1",
                    "alya_rodata_start",
                    os,
                );
                self.output.push_str("    cmp x0, x1\n");
                self.output.push_str(&format!("    b.lo {}\n", l_not_ro));
                crate::codegen::arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x2",
                    "alya_rodata_end",
                    os,
                );
                self.output.push_str("    cmp x0, x2\n");
                self.output.push_str(&format!("    b.lo {}\n", l_str));
                self.output.push_str(&format!("{}:\n", l_not_ro));
                crate::codegen::arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x1",
                    "alya_str_buf",
                    os,
                );
                self.output.push_str("    cmp x0, x1\n");
                self.output.push_str(&format!("    b.lo {}\n", l_stable));
                self.output.push_str("    movz x2, #1024, lsl #16\n");
                self.output.push_str("    add x2, x1, x2\n");
                self.output.push_str("    cmp x0, x2\n");
                self.output.push_str(&format!("    b.hs {}\n", l_stable));
                self.output.push_str("    ldrb w2, [x0]\n");
                self.output.push_str(&format!("    cbz w2, {}\n", l_int));
                self.output.push_str(&format!("    b {}\n", l_str));
                // Stable store region (B1): immortal strings live here
                // whole-program — same string treatment as str_buf above.
                self.output.push_str(&format!("{}:\n", l_stable));
                // Stable store region (B1): immortal strings live here
                // whole-program — same string treatment as str_buf above.
                crate::codegen::arch::arm64::emit_adrp_add(
                    &mut self.output,
                    "x1",
                    "alya_str_stable",
                    os,
                );
                self.output.push_str("    cmp x0, x1\n");
                self.output.push_str(&format!("    b.lo {}\n", l_int));
                self.output.push_str("    mov x2, #64\n");
                self.output.push_str("    lsl x2, x2, #20\n");
                self.output.push_str("    add x2, x1, x2\n");
                self.output.push_str("    cmp x0, x2\n");
                self.output.push_str(&format!("    b.hs {}\n", l_int));
                self.output.push_str("    ldrb w2, [x0]\n");
                self.output.push_str(&format!("    cbz w2, {}\n", l_int));
                self.output.push_str(&format!("    b {}\n", l_str));
                self.output.push_str(&format!("{}:\n", l_int));
                self.output
                    .push_str(&format!("    mov x0, #{}\n", KIND_INT));
                self.output.push_str(&format!("    b {}\n", l_end));
                self.output.push_str(&format!("{}:\n", l_str));
                self.output
                    .push_str(&format!("    mov x0, #{}\n", KIND_STRING));
                self.output.push_str(&format!("    b {}\n", l_end));
                self.output.push_str(&format!("{}:\n", l_end));
            }
        }
        let _ = os;
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
        let temp_offset = self.temp_offset();
        self.ctx.stack_offset += temp_offset;

        if is_string_expr(right, &self.ctx.variables) {
            self.generate_expression(right);
        } else {
            self.generate_expression(&Expr::Call {
                name: "str".into(),
                args: vec![right.clone()],
            });
        }
        self.ctx.stack_offset -= temp_offset;
        arch::emit_string_concat_call(&mut self.output, self.arch, self.ctx.stack_offset, self.os);
    }

    pub(crate) fn generate_string_equality(&mut self, left: &Expr, right: &Expr, op: BinaryOp) {
        self.generate_expression(left);
        arch::emit_push_temp(&mut self.output, self.arch);
        let temp_offset = self.temp_offset();
        self.ctx.stack_offset += temp_offset;

        self.generate_expression(right);
        self.ctx.stack_offset -= temp_offset;
        arch::emit_string_equality_call(
            &mut self.output,
            self.arch,
            op,
            self.ctx.stack_offset,
            self.os,
        );
    }

    /// Equality with runtime classification for operands without a proven
    /// static type (issues #41/#44).
    ///
    /// Both values are pushed (stack top-first: right, left), then each
    /// side is classified (class 3 from `emit_runtime_classify` means
    /// string). Both strings -> content compare via `fn_strcmp`;
    /// otherwise -> word compare. Both sides are always verified because
    /// a singly-marked string side cannot be trusted: bare-name inference
    /// markers collide across functions, so the marking may be wrong and
    /// trusting it would fold a true equality to constant-false.
    /// Pushes and pops balance on every path; `stack_offset` accounting
    /// mirrors `generate_string_equality` so call padding stays correct.
    pub(crate) fn generate_dynamic_equality(&mut self, left: &Expr, right: &Expr, op: BinaryOp) {
        let is_eq = matches!(op, BinaryOp::Equal);
        self.generate_expression(left);
        arch::emit_push_temp(&mut self.output, self.arch);
        let temp_offset = self.temp_offset();
        self.ctx.stack_offset += temp_offset;

        self.generate_expression(right);
        self.ctx.stack_offset -= temp_offset;
        arch::emit_push_temp(&mut self.output, self.arch);

        let l_not_str = self.ctx.next_label();
        let l_end = self.ctx.next_label();
        match self.arch {
            Architecture::X64 => {
                self.output.push_str("    movq (%rsp), %rax\n");
                self.emit_runtime_classify(self.os);
                self.output.push_str("    movq %rax, %rbx\n");
                self.output.push_str("    movq 8(%rsp), %rax\n");
                self.emit_runtime_classify(self.os);
                self.output.push_str("    cmp $3, %rax\n");
                self.output.push_str(&format!("    jne {}\n", l_not_str));
                self.output.push_str("    cmp $3, %rbx\n");
                self.output.push_str(&format!("    jne {}\n", l_not_str));
                self.output.push_str("    pop %rax\n");
                arch::emit_string_equality_call(
                    &mut self.output,
                    self.arch,
                    op,
                    self.ctx.stack_offset,
                    self.os,
                );
                arch::emit_jump(&mut self.output, self.arch, &l_end);
                self.output.push_str(&format!("{}:\n", l_not_str));
                self.output.push_str("    pop %rbx\n");
                self.output.push_str("    pop %rax\n");
                self.output.push_str("    cmp %rbx, %rax\n");
                if is_eq {
                    self.output.push_str("    sete %al\n");
                } else {
                    self.output.push_str("    setne %al\n");
                }
                self.output.push_str("    movzbq %al, %rax\n");
                self.output.push_str(&format!("{}:\n", l_end));
            }
            Architecture::ARM64 => {
                self.output.push_str("    ldr x0, [sp]\n");
                self.emit_runtime_classify(self.os);
                self.output.push_str("    mov x9, x0\n");
                self.output.push_str("    ldr x0, [sp, #16]\n");
                self.emit_runtime_classify(self.os);
                self.output
                    .push_str(&format!("    cmp x0, #{}\n", KIND_STRING));
                self.output.push_str(&format!("    b.ne {}\n", l_not_str));
                self.output
                    .push_str(&format!("    cmp x9, #{}\n", KIND_STRING));
                self.output.push_str(&format!("    b.ne {}\n", l_not_str));
                self.output.push_str("    ldr x0, [sp], #16\n");
                arch::emit_string_equality_call(
                    &mut self.output,
                    self.arch,
                    op,
                    self.ctx.stack_offset,
                    self.os,
                );
                arch::emit_jump(&mut self.output, self.arch, &l_end);
                self.output.push_str(&format!("{}:\n", l_not_str));
                self.output.push_str("    ldr x1, [sp], #16\n");
                self.output.push_str("    ldr x0, [sp], #16\n");
                self.output.push_str("    cmp x0, x1\n");
                if is_eq {
                    self.output.push_str("    cset x0, eq\n");
                } else {
                    self.output.push_str("    cset x0, ne\n");
                }
                self.output.push_str(&format!("{}:\n", l_end));
            }
        }
    }

    /// Resolves a field access to its defining struct and field index,
    /// sharing exact and fuzzy precedence (exact definition match, then
    /// name-similarity, then any-holder fallback). Layout queries must use
    /// this (not a bare index) so offsets stay consistent with the
    /// definition the index came from (alya-lang/alya#62).
    pub(crate) fn resolve_struct_field_target(
        &self,
        object: &Expr,
        field: &str,
    ) -> Option<(String, usize)> {
        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            Expr::ForceUnwrap(inner) => inner.as_ref(),
            _ => object,
        };

        // 1. Precise recursive type resolution
        if let Some(struct_name) = self.get_expr_struct_name(base_obj) {
            let bare = struct_name.rsplit("::").next().unwrap_or(&struct_name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            let found = self
                .ctx
                .structs
                .get_key_value(&struct_name)
                .or_else(|| self.ctx.structs.get_key_value(bare));
            if let Some((key, sdef)) = found {
                if let Some(idx) = sdef.fields.iter().position(|f| f == field) {
                    return Some((key.clone(), idx));
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
                return Some((matches[0].1.clone(), matches[0].2));
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
            return Some((candidates[0].0.clone(), candidates[0].1));
        }

        None
    }

    pub(crate) fn is_struct_field_weak(&self, object: &Expr, field: &str) -> bool {
        let base_obj = match object {
            Expr::OptionalFieldAccess { object: inner, .. } => inner.as_ref(),
            Expr::ForceUnwrap(inner) => inner.as_ref(),
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

        for sdef in self.ctx.structs.values() {
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

    pub(crate) fn get_type_size_and_align(
        type_name: &str,
        structs: &std::collections::HashMap<String, crate::codegen::context::StructDefInfo>,
    ) -> (i64, i64) {
        match type_name {
            "i8" | "u8" | "byte" | "bool" => (1, 1),
            "i16" | "u16" => (2, 2),
            "i32" | "u32" | "int32" | "uint32" | "f32" | "float32" => (4, 4),
            "i64" | "u64" | "int" | "uint" | "f64" | "float" | "rune" | "string" | "str" => (8, 8),
            _ => {
                let bare = type_name.rsplit("::").next().unwrap_or(type_name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = structs.get(type_name).or_else(|| structs.get(bare)) {
                    let mut current_offset: i64 = 0;
                    let mut max_align: i64 = 1;
                    for ftype_opt in &sdef.field_types {
                        let ftype = ftype_opt.as_deref().unwrap_or("int");
                        let (fsize, falign) = Self::get_type_size_and_align(ftype, structs);
                        if falign > max_align {
                            max_align = falign;
                        }
                        if falign > 0 && current_offset % falign != 0 {
                            current_offset += falign - (current_offset % falign);
                        }
                        current_offset += fsize;
                    }
                    if max_align > 0 && current_offset % max_align != 0 {
                        current_offset += max_align - (current_offset % max_align);
                    }
                    (current_offset, max_align)
                } else {
                    (8, 8)
                }
            }
        }
    }
}
