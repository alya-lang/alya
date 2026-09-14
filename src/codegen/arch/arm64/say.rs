use super::emit_adrp_add;
use super::loads::{emit_arm64_load_x29_offset, emit_load_reg_imm64};
use crate::codegen::target::OperatingSystem;

pub fn emit_call_printf(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) {
        "_"
    } else {
        ""
    };
    out.push_str(&format!("    bl {}printf\n", p));
}

pub fn emit_say_str(out: &mut String, label: &str, fmt_label: &str, os: OperatingSystem) {
    emit_adrp_add(out, "x1", label, os);
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        emit_call_printf(out, os);
        out.push_str("    add sp, sp, #16\n");
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_say_str_lit(out: &mut String, label: &str, os: OperatingSystem) {
    emit_adrp_add(out, "x0", label, os);
    emit_call_printf(out, os);
}

pub fn emit_say_offset(
    out: &mut String,
    offset: i32,
    _stack_offset: i32,
    fmt_label: &str,
    os: OperatingSystem,
) {
    emit_arm64_load_x29_offset(out, "x1", offset, "x9");
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        emit_call_printf(out, os);
        out.push_str("    add sp, sp, #16\n");
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_say_num_const(out: &mut String, val: i64, fmt_label: &str, os: OperatingSystem) {
    emit_load_reg_imm64(out, "x1", val);
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        emit_call_printf(out, os);
        out.push_str("    add sp, sp, #16\n");
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_say_acc(out: &mut String, fmt_label: &str, os: OperatingSystem) {
    out.push_str("    mov x1, x0\n");
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        emit_call_printf(out, os);
        out.push_str("    add sp, sp, #16\n");
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_say_float(out: &mut String, fmt_label: &str, os: OperatingSystem) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    fmov d0, x0\n");
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        emit_call_printf(out, os);
        out.push_str("    add sp, sp, #16\n");
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_say_interpolated_pop_and_call(
    out: &mut String,
    fmt_label: &str,
    is_floats: &[bool],
    os: OperatingSystem,
) {
    let count = is_floats.len();
    for i in (0..count).rev() {
        out.push_str(&format!("    ldr x{}, [sp], #16\n", i + 1));
    }
    if !matches!(os, OperatingSystem::MacOS) {
        let mut d_idx = 0;
        for (i, &is_flt) in is_floats.iter().enumerate() {
            if is_flt && d_idx < 8 {
                out.push_str(&format!("    fmov d{}, x{}\n", d_idx, i + 1));
                d_idx += 1;
            }
        }
    }
    emit_adrp_add(out, "x0", fmt_label, os);
    if matches!(os, OperatingSystem::MacOS) {
        let stack_space = (count * 8).div_ceil(16) * 16;
        out.push_str(&format!("    sub sp, sp, #{}\n", stack_space));
        for i in 0..count {
            out.push_str(&format!("    str x{}, [sp, #{}]\n", i + 1, i * 8));
        }
        emit_call_printf(out, os);
        out.push_str(&format!("    add sp, sp, #{}\n", stack_space));
    } else {
        emit_call_printf(out, os);
    }
}

pub fn emit_string_concat_call(out: &mut String) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    ldr x0, [sp], #16\n");
    out.push_str("    bl alya_concat\n");
}
