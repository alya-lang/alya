mod assign;
mod control;

use super::CodeGen;
use crate::ast::{Expr, Stmt};
use crate::codegen::arch;
use crate::codegen::context::VarType;
use crate::codegen::target::Architecture;

impl CodeGen {
    pub(crate) fn generate_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import { .. } | Stmt::ExternBlock { .. } | Stmt::Const { .. } => {}
            Stmt::Say(expr) => self.generate_say(expr),
            Stmt::Expr(expr) => {
                // Discarded heap temporaries: release fresh-owned results
                // (literals, direct struct constructors, freshness-marked calls)
                // immediately after evaluating (alya-lang/alya#81). Borrow-returning
                // calls and non-heap expressions are skipped to prevent use-after-free.
                let drops_fresh_heap = self.is_heap_expression(expr)
                    && (matches!(
                        expr,
                        Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. }
                    ) || match expr {
                        Expr::Call { name, .. } => {
                            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            self.ctx.structs.contains_key(name)
                                || self.ctx.structs.contains_key(bare)
                                || crate::codegen::analysis::call_returns_fresh_value(
                                    name,
                                    &self.ctx.variables,
                                )
                        }
                        _ => false,
                    });
                self.generate_expression(expr);
                if drops_fresh_heap {
                    arch::emit_rc_release(
                        &mut self.output,
                        self.arch,
                        self.ctx.stack_offset,
                        self.os,
                    );
                }
            }
            Stmt::Let {
                name,
                type_ann,
                value,
            } => self.generate_let(name, type_ann.as_deref(), value),
            Stmt::Assign { name, value } => {
                // Bare assignment to a new name declares it (spec
                // §variables, alya-lang/alya#82): route through
                // `generate_let` so the value gets a home slot and a
                // registered type. Otherwise the value is dropped and
                // later reads hit arbitrary slots (empty output or
                // garbage depending on surrounding code).
                if !self.ctx.variables.contains_key(name) && !self.ctx.globals.contains_key(name) {
                    self.generate_let(name, None, value);
                } else {
                    self.generate_assign(name, value);
                }
            }
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                self.generate_field_assign(object, field, value);
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                self.generate_index_assign(array, index, value);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => self.generate_if(condition, then_block, else_block.as_deref()),
            Stmt::While { condition, body } => self.generate_while(condition, body),
            Stmt::Repeat { body } => self.generate_repeat(body),
            Stmt::For {
                var,
                start,
                end,
                inclusive,
                body,
            } => self.generate_for(var, start, end, *inclusive, body),
            Stmt::ForEach {
                var,
                value_var,
                iterable,
                body,
            } => self.generate_for_each(var, value_var.as_deref(), iterable, body),
            Stmt::Break => {
                if let Some((_, break_label, base_offset)) = self.ctx.current_loop().cloned() {
                    let delta = self.ctx.stack_offset - base_offset;
                    if delta > 0 {
                        arch::emit_stack_restore(&mut self.output, self.arch, delta);
                    }
                    arch::emit_jump(&mut self.output, self.arch, &break_label);
                }
            }
            Stmt::Continue => {
                if let Some((continue_label, _, base_offset)) = self.ctx.current_loop().cloned() {
                    let delta = self.ctx.stack_offset - base_offset;
                    if delta > 0 {
                        arch::emit_stack_restore(&mut self.output, self.arch, delta);
                    }
                    arch::emit_jump(&mut self.output, self.arch, &continue_label);
                }
            }
            Stmt::Return(opt_expr) => {
                let mut skip_offset = None;
                if let Some(expr) = opt_expr {
                    let is_flt = crate::codegen::analysis::is_float_expr(expr, &self.ctx.variables)
                        || {
                            // A `-> float` function promises its result in
                            // the float return register even when the
                            // returned expression is not itself inferred
                            // float (e.g. an array element read). Callers
                            // sync via `fn_ret_flt` unconditionally, so the
                            // callee must honor the same marking or callers
                            // read stale d0/xmm0 (alya-lang/alya#51).
                            let cur = self.ctx.current_fn_name.clone();
                            let bare = cur.rsplit("::").next().unwrap_or(&cur);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            self.ctx
                                .variables
                                .contains_key(&format!("fn_ret_flt:{}", cur))
                                || self
                                    .ctx
                                    .variables
                                    .contains_key(&format!("fn_ret_flt:{}", bare))
                        };
                    if let Expr::Identifier(id) = expr {
                        if let Some(
                            VarType::Array(off)
                            | VarType::Map(off)
                            | VarType::Struct { offset: off, .. }
                            | VarType::Interface { offset: off, .. },
                        ) = self.ctx.variables.get(id)
                        {
                            skip_offset = Some(*off);
                        }
                    }
                    self.generate_expression(expr);
                    // Borrowed heap returns (indexing, field access) must be
                    // retained before scope cleanup releases the container, preventing use-after-free.
                    let is_borrowed_container_access =
                        matches!(expr, Expr::Index { .. } | Expr::FieldAccess { .. });
                    let needs_return_retain = is_borrowed_container_access
                        && (self.is_heap_expression(expr) || self.store_value_needs_retain(expr));
                    if needs_return_retain {
                        arch::emit_rc_retain(
                            &mut self.output,
                            self.arch,
                            self.ctx.stack_offset,
                            self.os,
                        );
                    }
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        _ => 8,
                    };
                    // Return-tag protocol (Phase 2b, alya-lang/alya#39):
                    // qualifying functions leave (value, tag) for callers.
                    // Literal returns materialize their static kind now
                    // (reads already carry theirs); float literals also
                    // move the value into the int register.
                    let cur = self.ctx.current_fn_name.clone();
                    let bare = cur.rsplit("::").next().unwrap_or(&cur);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let ret_tagged = self
                        .ctx
                        .variables
                        .contains_key(&format!("fn_ret_tagged:{}", cur))
                        || self
                            .ctx
                            .variables
                            .contains_key(&format!("fn_ret_tagged:{}", bare));
                    if ret_tagged {
                        if let Expr::Float(_) = expr {
                            match self.arch {
                                Architecture::X64 => {
                                    self.output.push_str("    movq %xmm0, %rax\n");
                                }
                                Architecture::ARM64 => {
                                    self.output.push_str("    fmov x0, d0\n");
                                }
                            }
                        }
                        let lit_kind: Option<i64> = match expr {
                            Expr::Number(_) => Some(1),
                            Expr::String(_) => Some(3),
                            Expr::Float(_) => Some(2),
                            _ => None,
                        };
                        if let Some(kind) = lit_kind {
                            match self.arch {
                                Architecture::X64 => {
                                    self.output.push_str(&format!("    movl ${}, %edx\n", kind));
                                }
                                Architecture::ARM64 => {
                                    self.output.push_str(&format!("    mov w1, #{}\n", kind));
                                }
                            }
                        }
                    }
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset += word_size;
                    if ret_tagged {
                        // Spill the tag across defers/releases (calls
                        // clobber it); restored below. Mirrors the value
                        // spill shape above, including offset tracking.
                        match self.arch {
                            Architecture::X64 => {
                                self.output.push_str("    push %rdx\n");
                                self.ctx.stack_offset += 8;
                            }
                            Architecture::ARM64 => {
                                self.output.push_str("    str w1, [sp, #-16]!\n");
                                self.ctx.stack_offset += 16;
                            }
                        }
                    }

                    self.emit_run_defers();

                    let heap_offsets = self.get_scope_heap_offsets(skip_offset);
                    if !heap_offsets.is_empty() {
                        let check_match = skip_offset.is_none()
                            && matches!(expr, Expr::Ternary { .. } | Expr::NullCoalesce { .. });
                        self.emit_rc_release_scope_return(&heap_offsets, check_match);
                    }
                    self.ctx.stack_offset -= word_size;
                    if ret_tagged {
                        match self.arch {
                            Architecture::X64 => {
                                self.output.push_str("    pop %rdx\n");
                                self.ctx.stack_offset -= 8;
                            }
                            Architecture::ARM64 => {
                                self.output.push_str("    ldr w1, [sp], #16\n");
                                self.ctx.stack_offset -= 16;
                            }
                        }
                    }
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
                } else {
                    self.emit_run_defers();
                    self.emit_cleanup_scope(None);
                }
                arch::emit_function_epilogue(&mut self.output, self.arch);
            }
            Stmt::Defer(_inner) => {
                if let Some((_, _, offset)) = self.ctx.active_defers.get(self.ctx.next_defer_idx) {
                    let off = *offset;
                    self.ctx.next_defer_idx += 1;
                    arch::emit_load_num(&mut self.output, self.arch, 1);
                    arch::emit_store_var(&mut self.output, self.arch, off, self.ctx.stack_offset);
                }
            }
            Stmt::Throw(opt_expr) => self.generate_throw(opt_expr.as_ref()),
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => self.generate_try_catch(
                try_block,
                catch_var.as_deref(),
                catch_block,
                finally_block.as_deref(),
            ),
            Stmt::Function { .. } => {}
            Stmt::StructDef {
                name,
                fields,
                field_types,
                defaults,
                ..
            } => {
                self.ctx.structs.insert(
                    name.clone(),
                    crate::codegen::context::StructDefInfo {
                        name: name.clone(),
                        fields: fields.clone(),
                        field_types: field_types.clone(),
                        defaults: defaults.clone(),
                    },
                );
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    self.ctx.structs.insert(
                        bare.to_string(),
                        crate::codegen::context::StructDefInfo {
                            name: bare.to_string(),
                            fields: fields.clone(),
                            field_types: field_types.clone(),
                            defaults: defaults.clone(),
                        },
                    );
                }
            }
            Stmt::EnumDef { .. } | Stmt::InterfaceDef { .. } => {}
            Stmt::Pub(inner) => self.generate_statement(inner),
        }
    }

    pub(crate) fn emit_run_defers(&mut self) {
        if self.ctx.active_defers.is_empty() {
            return;
        }
        let defers = self.ctx.active_defers.clone();
        for (_, inner_stmt, offset) in defers.into_iter().rev() {
            let skip_label = self.ctx.next_label();
            arch::emit_load_var(&mut self.output, self.arch, offset, self.ctx.stack_offset);
            arch::emit_cmp_imm(&mut self.output, self.arch, 0);
            arch::emit_cond_jump(
                &mut self.output,
                self.arch,
                crate::ast::BinaryOp::Equal,
                false,
                &skip_label,
            );
            self.generate_statement(&inner_stmt);
            self.output.push_str(&format!("{}:\n", skip_label));
        }
    }
}
