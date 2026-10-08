use super::{arm64, x64};
use crate::ast::BinaryOp;
use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_cmp_reg(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => out.push_str("    cmp x0, x1\n"),
        Architecture::X64 => out.push_str("    cmp %rbx, %rax\n"),
    }
}

pub fn emit_cmp_imm(out: &mut String, arch: Architecture, imm: i64) {
    match arch {
        Architecture::ARM64 => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                arm64::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
        }
        Architecture::X64 => {
            if (i32::MIN as i64..=i32::MAX as i64).contains(&imm) {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
        }
    }
}

pub fn emit_cond_jump(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    invert: bool,
    target: &str,
    unsigned: bool,
) {
    match arch {
        Architecture::ARM64 => {
            let cond = match (op, invert, unsigned) {
                (BinaryOp::Less, false, false) | (BinaryOp::GreaterEqual, true, false) => "b.lt",
                (BinaryOp::Less, true, false) | (BinaryOp::GreaterEqual, false, false) => "b.ge",
                (BinaryOp::LessEqual, false, false) | (BinaryOp::Greater, true, false) => "b.le",
                (BinaryOp::LessEqual, true, false) | (BinaryOp::Greater, false, false) => "b.gt",
                (BinaryOp::Less, false, true) | (BinaryOp::GreaterEqual, true, true) => "b.lo",
                (BinaryOp::Less, true, true) | (BinaryOp::GreaterEqual, false, true) => "b.hs",
                (BinaryOp::LessEqual, false, true) | (BinaryOp::Greater, true, true) => "b.ls",
                (BinaryOp::LessEqual, true, true) | (BinaryOp::Greater, false, true) => "b.hi",
                (BinaryOp::Equal, false, _) | (BinaryOp::NotEqual, true, _) => "b.eq",
                (BinaryOp::Equal, true, _) | (BinaryOp::NotEqual, false, _) => "b.ne",
                _ => "b.ne",
            };
            out.push_str(&format!("    {} {}\n", cond, target));
        }
        Architecture::X64 => {
            let jmp = match (op, invert, unsigned) {
                (BinaryOp::Less, false, false) | (BinaryOp::GreaterEqual, true, false) => "jl",
                (BinaryOp::Less, true, false) | (BinaryOp::GreaterEqual, false, false) => "jge",
                (BinaryOp::LessEqual, false, false) | (BinaryOp::Greater, true, false) => "jle",
                (BinaryOp::LessEqual, true, false) | (BinaryOp::Greater, false, false) => "jg",
                (BinaryOp::Less, false, true) | (BinaryOp::GreaterEqual, true, true) => "jb",
                (BinaryOp::Less, true, true) | (BinaryOp::GreaterEqual, false, true) => "jae",
                (BinaryOp::LessEqual, false, true) | (BinaryOp::Greater, true, true) => "jbe",
                (BinaryOp::LessEqual, true, true) | (BinaryOp::Greater, false, true) => "ja",
                (BinaryOp::Equal, false, _) | (BinaryOp::NotEqual, true, _) => "je",
                (BinaryOp::Equal, true, _) | (BinaryOp::NotEqual, false, _) => "jne",
                _ => "jne",
            };
            out.push_str(&format!("    {} {}\n", jmp, target));
        }
    }
}

pub fn emit_float_cmp_reg(out: &mut String, arch: Architecture) {
    // Same contract as emit_float_binary_op_reg: the left operand's raw f64
    // bits arrive in the integer return register, so sync the FP register
    // first (call/arithmetic results never touch it).
    match arch {
        Architecture::ARM64 => {
            out.push_str("    fmov d0, x0\n");
            out.push_str("    fcmp d0, d1\n")
        }
        Architecture::X64 => {
            out.push_str("    movq %rax, %xmm0\n");
            out.push_str("    ucomisd %xmm1, %xmm0\n")
        }
    }
}

pub fn emit_float_cond_jump(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    invert: bool,
    target: &str,
    skip: &str,
) {
    // NaN (#91): ucomisd/fcmp raise the unordered flag (x64 PF, arm64 V)
    // on NaN operands. Arms whose taken-condition excludes unordered
    // guard with jp/b.vs over the caller-provided skip label; arms that
    // take on unordered pair the ordered branch with jp/b.vs. Every other
    // combination is already IEEE-correct and stays single-branch.
    // (skip is only emitted by the guard arms; elsewhere it is unused.)
    match arch {
        Architecture::ARM64 => {
            match (op, invert) {
                // b.mi is N: ordered-less only; unordered (N=0) falls.
                (BinaryOp::Less, false) => {
                    out.push_str(&format!("    b.mi {}\n", target));
                }
                // !(a<b): N==V ordered, or unordered.
                (BinaryOp::Less, true) => {
                    out.push_str(&format!("    b.ge {}\n", target));
                    out.push_str(&format!("    b.vs {}\n", target));
                }
                // a<=b ordered, never unordered.
                (BinaryOp::LessEqual, false) => {
                    out.push_str(&format!("    b.vs {}\n", skip));
                    out.push_str(&format!("    b.le {}\n", target));
                    out.push_str(&format!("{}:\n", skip));
                }
                // !(a<=b): ordered-greater, or unordered.
                (BinaryOp::LessEqual, true) => {
                    out.push_str(&format!("    b.gt {}\n", target));
                    out.push_str(&format!("    b.vs {}\n", target));
                }
                // b.le is Z|(N!=V): exact for !(a>b), ordered or not.
                // (b.ls reads C, whose ordered-less value must not decide
                // an IEEE result; b.le depends on N/Z/V only.)
                (BinaryOp::Greater, true) => {
                    out.push_str(&format!("    b.le {}\n", target));
                }
                // b.gt is ~Z&(N==V): ordered-greater only.
                (BinaryOp::Greater, false) => {
                    out.push_str(&format!("    b.gt {}\n", target));
                }
                // b.ge is N==V: ordered only; unordered (N!=V) falls.
                (BinaryOp::GreaterEqual, false) => {
                    out.push_str(&format!("    b.ge {}\n", target));
                }
                // !(a>=b): N!=V ordered or not.
                (BinaryOp::GreaterEqual, true) => {
                    out.push_str(&format!("    b.lt {}\n", target));
                }
                // b.eq/b.ne read Z, which fcmp clears on unordered.
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => {
                    out.push_str(&format!("    b.eq {}\n", target));
                }
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => {
                    out.push_str(&format!("    b.ne {}\n", target));
                }
                _ => {
                    out.push_str(&format!("    b.ne {}\n", target));
                }
            }
        }
        Architecture::X64 => {
            match (op, invert) {
                // Taken iff below AND ordered.
                (BinaryOp::Less, false) => {
                    out.push_str(&format!("    jp {}\n", skip));
                    out.push_str(&format!("    jb {}\n", target));
                    out.push_str(&format!("{}:\n", skip));
                }
                // Taken iff not-below OR unordered.
                (BinaryOp::Less, true) => {
                    out.push_str(&format!("    jae {}\n", target));
                    out.push_str(&format!("    jp {}\n", target));
                }
                // Taken iff below-or-equal AND ordered.
                (BinaryOp::LessEqual, false) => {
                    out.push_str(&format!("    jp {}\n", skip));
                    out.push_str(&format!("    jbe {}\n", target));
                    out.push_str(&format!("{}:\n", skip));
                }
                // Taken iff above OR unordered.
                (BinaryOp::LessEqual, true) => {
                    out.push_str(&format!("    ja {}\n", target));
                    out.push_str(&format!("    jp {}\n", target));
                }
                // ja is CF=0&ZF=0: above only (unordered CF=1 falls).
                (BinaryOp::Greater, false) => {
                    out.push_str(&format!("    ja {}\n", target));
                }
                // Taken iff below-or-equal OR unordered (ZF=1 covers it).
                (BinaryOp::Greater, true) => {
                    out.push_str(&format!("    jbe {}\n", target));
                }
                // jae is CF=0: above-or-equal only (unordered CF=1 falls).
                (BinaryOp::GreaterEqual, false) => {
                    out.push_str(&format!("    jae {}\n", target));
                }
                // Taken iff below OR unordered: jb is CF=1 which covers
                // both (unordered sets CF=1). jl is wrong here because
                // ucomisd zeroes SF/OF, so jl never fires.
                (BinaryOp::GreaterEqual, true) => {
                    out.push_str(&format!("    jb {}\n", target));
                }
                // Taken iff equal AND ordered.
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => {
                    out.push_str(&format!("    jp {}\n", skip));
                    out.push_str(&format!("    je {}\n", target));
                    out.push_str(&format!("{}:\n", skip));
                }
                // Taken iff not-equal OR unordered.
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => {
                    out.push_str(&format!("    jne {}\n", target));
                    out.push_str(&format!("    jp {}\n", target));
                }
                _ => {
                    out.push_str(&format!("    jne {}\n", target));
                }
            }
        }
    }
}

pub fn emit_jump_if_zero(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump_if_zero(out, label),
        Architecture::X64 => x64::emit_jump_if_zero(out, label),
    }
}

pub fn emit_jump_if_not_zero(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump_if_not_zero(out, label),
        Architecture::X64 => x64::emit_jump_if_not_zero(out, label),
    }
}

pub fn emit_jump(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump(out, label),
        Architecture::X64 => x64::emit_jump(out, label),
    }
}

pub fn emit_compare_and_jump_if_greater(
    out: &mut String,
    arch: Architecture,
    label: &str,
    unsigned: bool,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_compare_and_jump_if_greater(out, label, unsigned),
        Architecture::X64 => x64::emit_compare_and_jump_if_greater(out, label, unsigned),
    }
}

pub fn emit_compare_and_jump_if_greater_or_equal(
    out: &mut String,
    arch: Architecture,
    label: &str,
    unsigned: bool,
) {
    match arch {
        Architecture::ARM64 => {
            arm64::emit_compare_and_jump_if_greater_or_equal(out, label, unsigned)
        }
        Architecture::X64 => x64::emit_compare_and_jump_if_greater_or_equal(out, label, unsigned),
    }
}

pub fn emit_increment_var(
    out: &mut String,
    arch: Architecture,
    var_offset: i32,
    stack_offset: i32,
    start_label: &str,
) {
    match arch {
        Architecture::ARM64 => {
            arm64::emit_increment_var(out, var_offset, stack_offset, start_label)
        }
        Architecture::X64 => x64::emit_increment_var(out, var_offset, start_label),
    }
}

pub fn mangle_symbol_name(name: &str) -> String {
    let s = if name.starts_with("_Alya_") {
        name.replace("::", "_")
    } else {
        name.replace("::", "__")
    };
    s.replace("operator[]=", "operator_index_assign_")
        .replace("operator[]", "operator_index_")
        .replace("operator-neg", "operator_neg_")
        .replace("operator==", "operator_eq_")
        .replace("operator!=", "operator_ne_")
        .replace("operator<=", "operator_le_")
        .replace("operator>=", "operator_ge_")
        .replace("operator<", "operator_lt_")
        .replace("operator>", "operator_gt_")
        .replace("operator+", "operator_add_")
        .replace("operator-", "operator_sub_")
        .replace("operator*", "operator_mul_")
        .replace("operator/", "operator_div_")
        .replace("operator%", "operator_mod_")
}

pub fn emit_function_prologue(
    out: &mut String,
    arch: Architecture,
    os: OperatingSystem,
    name: &str,
) {
    let mangled = mangle_symbol_name(name);
    match arch {
        Architecture::ARM64 => arm64::emit_function_prologue(out, &mangled, os),
        Architecture::X64 => x64::emit_function_prologue(out, &mangled, os),
    }
}

/// Emits a global alias label for `@export("name")` (Chapter 18 §1.2).
/// Must be called before the function prologue so both labels share one
/// address. The alias is emitted verbatim (no `fn_` prefix, no mangling).
pub fn emit_export_alias(out: &mut String, arch: Architecture, alias: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_export_alias(out, alias),
        Architecture::X64 => x64::emit_export_alias(out, alias),
    }
}

pub fn emit_function_epilogue(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_function_epilogue(out),
        Architecture::X64 => x64::emit_function_epilogue(out),
    }
}

/// Closes the Windows-x64 SEH scope for one generated function. No-op on
/// every other target (GAS ELF/Mach-O rejects `.seh_*`). Call once per
/// function at its final `.text` position.
pub fn emit_function_endproc(out: &mut String, arch: Architecture, os: OperatingSystem) {
    if matches!(arch, Architecture::X64) && matches!(os, OperatingSystem::Windows) {
        x64::emit_seh_endproc(out);
    }
}

pub fn emit_function_param_push(
    out: &mut String,
    arch: Architecture,
    param_idx: usize,
    stack_offset: &mut i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_function_param_push(out, param_idx, stack_offset),
        Architecture::X64 => x64::emit_function_param_push(out, param_idx, stack_offset, os),
    }
}

pub fn emit_function_call(
    out: &mut String,
    arch: Architecture,
    name: &str,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    let mangled = mangle_symbol_name(name);
    match arch {
        Architecture::ARM64 => arm64::emit_function_call(out, &mangled, args_count),
        Architecture::X64 => x64::emit_function_call(out, &mangled, args_count, stack_offset, os),
    }
}

pub fn emit_c_function_call(
    out: &mut String,
    arch: Architecture,
    name: &str,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    let mangled = mangle_symbol_name(name);
    match arch {
        Architecture::ARM64 => arm64::emit_c_function_call(out, &mangled, args_count, os),
        Architecture::X64 => x64::emit_c_function_call(out, &mangled, args_count, stack_offset, os),
    }
}

pub fn emit_indirect_function_call(
    out: &mut String,
    arch: Architecture,
    var_offset: i32,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_indirect_function_call(out, var_offset, args_count),
        Architecture::X64 => {
            x64::emit_indirect_function_call(out, var_offset, args_count, stack_offset, os)
        }
    }
}

pub fn emit_stack_restore(out: &mut String, arch: Architecture, delta: i32) {
    if delta <= 0 {
        return;
    }
    match arch {
        Architecture::ARM64 => arm64::emit_stack_restore(out, delta),
        Architecture::X64 => x64::emit_stack_restore(out, delta),
    }
}

/// Traps when a just-read value kind tag is float (alya-lang/alya#39
/// Phase 2b): call immediately after generating an untyped-param Index
/// read, while the tag is still fresh (x64: %edx, arm64: w1).
/// Non-float (int/unknown) tags fall through to integer semantics.
pub fn emit_mixed_float_check(out: &mut String, arch: Architecture) {
    use crate::codegen::kinds::KIND_FLOAT;
    match arch {
        Architecture::X64 => {
            out.push_str(&format!("    cmpl ${}, %edx\n", KIND_FLOAT));
            out.push_str("    je alya_error_mixed_float\n");
        }
        Architecture::ARM64 => {
            out.push_str(&format!("    cmp w1, #{}\n", KIND_FLOAT));
            out.push_str("    b.eq alya_error_mixed_float\n");
        }
    }
}
