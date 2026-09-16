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
            Stmt::Let { name, value } => self.generate_let(name, value),
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
                iterable,
                body,
            } => self.generate_for_each(var, iterable, body),
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
                    let heap_offsets = self.get_scope_heap_offsets(skip_offset);
                    if !heap_offsets.is_empty() {
                        arch::emit_push_temp(&mut self.output, self.arch);
                        for offset in heap_offsets {
                            arch::emit_rc_release_stack(
                                &mut self.output,
                                self.arch,
                                offset,
                                self.ctx.stack_offset + 8,
                                self.os,
                            );
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
                                Architecture::X86 => {}
                            }
                        }
                    }
                } else {
                    self.emit_cleanup_scope(None);
                }
                arch::emit_function_epilogue(&mut self.output, self.arch);
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
                defaults,
            } => {
                self.ctx.structs.insert(
                    name.clone(),
                    crate::codegen::context::StructDefInfo {
                        name: name.clone(),
                        fields: fields.clone(),
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
                            defaults: defaults.clone(),
                        },
                    );
                }
            }
            Stmt::EnumDef { .. } => {}
        }
    }
}
