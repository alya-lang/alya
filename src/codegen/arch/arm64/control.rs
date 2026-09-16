use super::loads::{emit_arm64_load_x29_offset, emit_arm64_store_x29_offset};
use crate::codegen::target::OperatingSystem;

pub fn emit_jump_if_zero(out: &mut String, label: &str) {
    out.push_str(&format!("    cbz x0, {}\n", label));
}

pub fn emit_jump_if_not_zero(out: &mut String, label: &str) {
    out.push_str(&format!("    cbnz x0, {}\n", label));
}

pub fn emit_jump(out: &mut String, label: &str) {
    out.push_str(&format!("    b {}\n", label));
}

pub fn emit_compare_and_jump_if_greater(out: &mut String, label: &str) {
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str("    cmp x1, x0\n");
    out.push_str(&format!("    b.gt {}\n", label));
}

pub fn emit_increment_var(
    out: &mut String,
    var_offset: i32,
    _stack_offset: i32,
    start_label: &str,
) {
    emit_arm64_load_x29_offset(out, "x0", var_offset, "x9");
    out.push_str("    add x0, x0, #1\n");
    emit_arm64_store_x29_offset(out, "x0", var_offset, "x9");
    out.push_str(&format!("    b {}\n", start_label));
}

pub fn emit_function_prologue(out: &mut String, name: &str) {
    out.push_str(&format!("\n.globl fn_{}\n", name));
    out.push_str(&format!("fn_{}:\n", name));
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n\n");
}

pub fn emit_function_param_push(out: &mut String, param_idx: usize, stack_offset: &mut i32) {
    *stack_offset += 16;
    if param_idx < 8 {
        let reg = match param_idx {
            0 => "x0",
            1 => "x1",
            2 => "x2",
            3 => "x3",
            4 => "x4",
            5 => "x5",
            6 => "x6",
            7 => "x7",
            _ => unreachable!(),
        };
        out.push_str(&format!("    str {}, [sp, #-16]!\n", reg));
    } else {
        let src_offset = 16 + (param_idx - 8) * 8;
        out.push_str(&format!("    ldr x9, [x29, #{}]\n", src_offset));
        out.push_str("    str x9, [sp, #-16]!\n");
    }
}

pub fn emit_function_epilogue(out: &mut String) {
    out.push_str("    mov sp, x29\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");
}

fn emit_call_target(out: &mut String, target: &str, args_count: usize) {
    if args_count <= 8 {
        for i in (0..args_count).rev() {
            let reg = match i {
                0 => "x0",
                1 => "x1",
                2 => "x2",
                3 => "x3",
                4 => "x4",
                5 => "x5",
                6 => "x6",
                7 => "x7",
                _ => unreachable!(),
            };
            out.push_str(&format!("    ldr {}, [sp], #16\n", reg));
        }
        out.push_str(&format!("    bl {}\n", target));
    } else {
        let extra_args = args_count - 8;
        let needed = extra_args as i32 * 8;
        let total_alloc = if needed % 16 == 0 { needed } else { needed + 8 };

        for i in 0..8 {
            let offset = (args_count - 1 - i) * 16;
            out.push_str(&format!("    ldr x{}, [sp, #{}]\n", i, offset));
        }

        out.push_str(&format!("    sub sp, sp, #{}\n", total_alloc));

        for k in 8..args_count {
            let src_off = total_alloc + ((args_count - 1 - k) * 16) as i32;
            let dst_off = ((k - 8) * 8) as i32;
            out.push_str(&format!("    ldr x9, [sp, #{}]\n", src_off));
            out.push_str(&format!("    str x9, [sp, #{}]\n", dst_off));
        }

        out.push_str(&format!("    bl {}\n", target));

        let total_restore = total_alloc + args_count as i32 * 16;
        out.push_str(&format!("    add sp, sp, #{}\n", total_restore));
    }
}

pub fn emit_function_call(out: &mut String, name: &str, args_count: usize) {
    emit_call_target(out, &format!("fn_{}", name), args_count);
}

pub fn emit_c_function_call(out: &mut String, name: &str, args_count: usize, os: OperatingSystem) {
    let target = if matches!(os, OperatingSystem::MacOS) {
        format!("_{}", name)
    } else {
        name.to_string()
    };
    emit_call_target(out, &target, args_count);
}

pub fn emit_stack_restore(out: &mut String, delta: i32) {
    out.push_str(&format!("    add sp, sp, #{}\n", delta));
}
