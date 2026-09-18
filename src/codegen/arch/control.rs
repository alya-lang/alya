use super::{arm64, x64, x86};
use crate::ast::BinaryOp;
use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_cmp_reg(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => out.push_str("    cmp x0, x1\n"),
        Architecture::X64 => out.push_str("    cmp %rbx, %rax\n"),
        Architecture::X86 => out.push_str("    cmp %ebx, %eax\n"),
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
        Architecture::X86 => {
            out.push_str(&format!("    cmp ${}, %eax\n", imm as i32));
        }
    }
}

pub fn emit_cond_jump(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    invert: bool,
    target: &str,
) {
    match arch {
        Architecture::ARM64 => {
            let cond = match (op, invert) {
                (BinaryOp::Less, false) | (BinaryOp::GreaterEqual, true) => "b.lt",
                (BinaryOp::Less, true) | (BinaryOp::GreaterEqual, false) => "b.ge",
                (BinaryOp::LessEqual, false) | (BinaryOp::Greater, true) => "b.le",
                (BinaryOp::LessEqual, true) | (BinaryOp::Greater, false) => "b.gt",
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => "b.eq",
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => "b.ne",
                _ => "b.ne",
            };
            out.push_str(&format!("    {} {}\n", cond, target));
        }
        Architecture::X64 | Architecture::X86 => {
            let jmp = match (op, invert) {
                (BinaryOp::Less, false) | (BinaryOp::GreaterEqual, true) => "jl",
                (BinaryOp::Less, true) | (BinaryOp::GreaterEqual, false) => "jge",
                (BinaryOp::LessEqual, false) | (BinaryOp::Greater, true) => "jle",
                (BinaryOp::LessEqual, true) | (BinaryOp::Greater, false) => "jg",
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => "je",
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => "jne",
                _ => "jne",
            };
            out.push_str(&format!("    {} {}\n", jmp, target));
        }
    }
}

pub fn emit_float_cmp_reg(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => out.push_str("    fcmp d0, d1\n"),
        Architecture::X64 | Architecture::X86 => out.push_str("    ucomisd %xmm1, %xmm0\n"),
    }
}

pub fn emit_float_cond_jump(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    invert: bool,
    target: &str,
) {
    match arch {
        Architecture::ARM64 => {
            let cond = match (op, invert) {
                (BinaryOp::Less, false) | (BinaryOp::GreaterEqual, true) => "b.mi",
                (BinaryOp::Less, true) | (BinaryOp::GreaterEqual, false) => "b.ge",
                (BinaryOp::LessEqual, false) | (BinaryOp::Greater, true) => "b.ls",
                (BinaryOp::LessEqual, true) | (BinaryOp::Greater, false) => "b.gt",
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => "b.eq",
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => "b.ne",
                _ => "b.ne",
            };
            out.push_str(&format!("    {} {}\n", cond, target));
        }
        Architecture::X64 | Architecture::X86 => {
            let jmp = match (op, invert) {
                (BinaryOp::Less, false) | (BinaryOp::GreaterEqual, true) => "jb",
                (BinaryOp::Less, true) | (BinaryOp::GreaterEqual, false) => "jae",
                (BinaryOp::LessEqual, false) | (BinaryOp::Greater, true) => "jbe",
                (BinaryOp::LessEqual, true) | (BinaryOp::Greater, false) => "ja",
                (BinaryOp::Equal, false) | (BinaryOp::NotEqual, true) => "je",
                (BinaryOp::Equal, true) | (BinaryOp::NotEqual, false) => "jne",
                _ => "jne",
            };
            out.push_str(&format!("    {} {}\n", jmp, target));
        }
    }
}

pub fn emit_jump_if_zero(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump_if_zero(out, label),
        Architecture::X64 => x64::emit_jump_if_zero(out, label),
        Architecture::X86 => x86::emit_jump_if_zero(out, label),
    }
}

pub fn emit_jump_if_not_zero(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump_if_not_zero(out, label),
        Architecture::X64 => x64::emit_jump_if_not_zero(out, label),
        Architecture::X86 => x86::emit_jump_if_not_zero(out, label),
    }
}

pub fn emit_jump(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_jump(out, label),
        Architecture::X64 => x64::emit_jump(out, label),
        Architecture::X86 => x86::emit_jump(out, label),
    }
}

pub fn emit_compare_and_jump_if_greater(out: &mut String, arch: Architecture, label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_compare_and_jump_if_greater(out, label),
        Architecture::X64 => x64::emit_compare_and_jump_if_greater(out, label),
        Architecture::X86 => x86::emit_compare_and_jump_if_greater(out, label),
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
        Architecture::X86 => x86::emit_increment_var(out, var_offset, start_label),
    }
}

pub fn mangle_symbol_name(name: &str) -> String {
    let s = name.replace("::", "__");
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

pub fn emit_function_prologue(out: &mut String, arch: Architecture, name: &str) {
    let mangled = mangle_symbol_name(name);
    match arch {
        Architecture::ARM64 => arm64::emit_function_prologue(out, &mangled),
        Architecture::X64 => x64::emit_function_prologue(out, &mangled),
        Architecture::X86 => x86::emit_function_prologue(out, &mangled),
    }
}

pub fn emit_function_epilogue(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_function_epilogue(out),
        Architecture::X64 => x64::emit_function_epilogue(out),
        Architecture::X86 => x86::emit_function_epilogue(out),
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
        Architecture::X86 => x86::emit_function_param_push(out, param_idx, stack_offset),
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
        Architecture::X86 => x86::emit_function_call(out, &mangled, args_count),
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
        Architecture::X86 => x86::emit_c_function_call(out, &mangled, args_count),
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
        Architecture::X86 => x86::emit_indirect_function_call(out, var_offset, args_count),
    }
}

pub fn emit_stack_restore(out: &mut String, arch: Architecture, delta: i32) {
    if delta <= 0 {
        return;
    }
    match arch {
        Architecture::ARM64 => arm64::emit_stack_restore(out, delta),
        Architecture::X64 => x64::emit_stack_restore(out, delta),
        Architecture::X86 => x86::emit_stack_restore(out, delta),
    }
}
