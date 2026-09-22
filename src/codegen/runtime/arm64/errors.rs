use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_exit
    out.push_str("fn_exit:\n");
    out.push_str(&format!("    b {}exit\n\n", p));

    // fn_throw
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    emit_adrp_add(out, "x9", "alya_err_msg", os);
    out.push_str("    str x0, [x9]\n");
    emit_adrp_add(out, "x9", "alya_catch_idx", os);
    out.push_str("    ldr x10, [x9]\n");
    out.push_str("    cbz x10, .L_arm_fatal_throw\n");
    out.push_str("    sub x10, x10, #1\n");
    out.push_str("    str x10, [x9]\n");
    emit_adrp_add(out, "x11", "alya_catch_stack_sp", os);
    out.push_str("    ldr x13, [x11, x10, lsl #3]\n");
    out.push_str("    mov sp, x13\n");
    emit_adrp_add(out, "x11", "alya_catch_stack_bp", os);
    out.push_str("    ldr x29, [x11, x10, lsl #3]\n");
    emit_adrp_add(out, "x11", "alya_catch_stack_handler", os);
    out.push_str("    ldr x14, [x11, x10, lsl #3]\n");
    out.push_str("    br x14\n");
    out.push_str(".L_arm_fatal_throw:\n");
    out.push_str("    mov x19, sp\n");
    out.push_str("    bic x19, x19, #15\n");
    out.push_str("    mov sp, x19\n");
    out.push_str("    mov x1, x0\n");
    emit_adrp_add(out, "x0", "alya_fmt_runtime_err", os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        out.push_str(&format!("    bl {}printf\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}printf\n", p));
    }
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    bl {}fflush\n", p));
    out.push_str("    mov w0, #1\n");
    out.push_str(&format!("    bl {}exit\n\n", p));

    // fn_rethrow
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    emit_adrp_add(out, "x9", "alya_err_msg", os);
    out.push_str("    ldr x0, [x9]\n");
    out.push_str("    cbnz x0, .L_arm_rethrow_has_msg\n");
    emit_adrp_add(out, "x0", "alya_str_unhandled_err", os);
    out.push_str(".L_arm_rethrow_has_msg:\n");
    out.push_str("    b fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    emit_adrp_add(out, "x0", "alya_str_div_zero", os);
    out.push_str("    b fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    emit_adrp_add(out, "x0", "alya_str_bounds", os);
    out.push_str("    b fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    emit_adrp_add(out, "x0", "alya_str_null_unwrap", os);
    out.push_str("    b fn_throw\n\n");

    // fn_sleep
    out.push_str(".align 2\n");
    out.push_str(".global fn_sleep\n");
    out.push_str("fn_sleep:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x1, #1000\n");
    out.push_str("    mul x0, x0, x1\n");
    out.push_str(&format!("    bl {}usleep\n", p));
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_system_exec
    out.push_str(".align 2\n");
    out.push_str(".global fn_system_exec\n");
    out.push_str("fn_system_exec:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_sysexec_empty\n");
    out.push_str(&format!("    bl {}system\n", p));
    out.push_str("    b .L_arm64_sysexec_ret\n");
    out.push_str(".L_arm64_sysexec_empty:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_sysexec_ret:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_get_pid
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_get_pid\n");
    out.push_str("fn___native_get_pid:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(&format!("    bl {}getpid\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_get_cwd
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_get_cwd\n");
    out.push_str("fn___native_get_cwd:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    sub sp, sp, #1024\n");
    out.push_str("    mov x0, sp\n");
    out.push_str("    mov x1, #1024\n");
    out.push_str(&format!("    bl {}getcwd\n", p));
    out.push_str("    cbz x0, .L_arm64_gcwd_empty\n");
    out.push_str("    bl fn_str_clone\n");
    out.push_str("    b .L_arm64_gcwd_done\n");
    out.push_str(".L_arm64_gcwd_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_gcwd_done:\n");
    out.push_str("    add sp, sp, #1024\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_set_cwd
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_set_cwd\n");
    out.push_str("fn___native_set_cwd:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_scwd_fail\n");
    out.push_str(&format!("    bl {}chdir\n", p));
    out.push_str("    cmp x0, #0\n");
    out.push_str("    cset x0, eq\n");
    out.push_str("    b .L_arm64_scwd_done\n");
    out.push_str(".L_arm64_scwd_fail:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_scwd_done:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");
}
