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
                self.generate_expression(expr);
            }
            Stmt::Let {
                name,
                type_ann,
                value,
            } => self.generate_let(name, type_ann.as_deref(), value),
            Stmt::Assign { name, value } => self.generate_assign(name, value),
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
                body,
            } => self.generate_for(var, start, end, body),
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
                    let is_flt = crate::codegen::analysis::is_float_expr(expr, &self.ctx.variables);
                    if let Expr::Identifier(id) = expr {
                        if let Some(
                            VarType::Array(off)
                            | VarType::Map(off)
                            | VarType::Struct { offset: off, .. },
                        ) = self.ctx.variables.get(id)
                        {
                            skip_offset = Some(*off);
                        }
                    }
                    self.generate_expression(expr);
                    let word_size: i32 = match self.arch {
                        Architecture::ARM64 => 16,
                        Architecture::X86 => 4,
                        _ => 8,
                    };
                    arch::emit_push_temp(&mut self.output, self.arch);
                    self.ctx.stack_offset += word_size;

                    self.emit_run_defers();

                    let heap_offsets = self.get_scope_heap_offsets(skip_offset);
                    if !heap_offsets.is_empty() {
                        let check_match = skip_offset.is_none()
                            && matches!(expr, Expr::Ternary { .. } | Expr::NullCoalesce { .. });
                        self.emit_rc_release_scope_return(&heap_offsets, check_match);
                    }
                    self.ctx.stack_offset -= word_size;
                    arch::emit_pop_temp(&mut self.output, self.arch);
                    if is_flt {
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
