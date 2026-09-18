use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_struct_new
    out.push_str("alya_struct_new:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    add x0, x20, #3\n");
    out.push_str("    mov x1, #8\n");
    out.push_str(&format!("    bl {}calloc\n", p));
    out.push_str("    movz x1, #0x0003\n");
    out.push_str("    movk x1, #0x5A11, lsl #16\n");
    out.push_str("    str x1, [x0]\n");
    out.push_str("    mov x1, #1\n");
    out.push_str("    str x1, [x0, #8]\n");
    out.push_str("    add x0, x0, #16\n");
    out.push_str("    str x19, [x0]\n");
    out.push_str("    add x1, x20, #3\n");
    out.push_str("    lsl x1, x1, #3\n");
    emit_adrp_add(out, "x2", "alya_allocated_bytes", os);
    out.push_str("    ldr x3, [x2]\n");
    out.push_str("    add x3, x3, x1\n");
    out.push_str("    str x3, [x2]\n");
    out.push_str("    mov x2, #3\n");
    out.push_str("    mov x3, x19\n");
    out.push_str("    stp x0, x1, [sp, #-16]!\n");
    out.push_str("    bl alya_mem_track_alloc\n");
    out.push_str("    ldp x0, x1, [sp], #16\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // alya_print_struct
    out.push_str("alya_print_struct:\n");
    out.push_str("    stp x29, x30, [sp, #-64]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    str x23, [sp, #48]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    cbnz x19, .L_arm64_struct_not_null\n");
    emit_adrp_add(out, "x0", "alya_fmt_struct_null", os);
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_struct_exit\n");
    out.push_str(".L_arm64_struct_not_null:\n");
    out.push_str("    ldr x20, [x19]\n");
    out.push_str("    ldr x21, [x20, #8]\n");
    emit_adrp_add(out, "x0", "alya_fmt_struct_open", os);
    out.push_str("    ldr x1, [x20]\n");
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        out.push_str(&format!("    bl {}printf\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}printf\n", p));
    }
    out.push_str("    mov x22, #0\n");
    out.push_str(".L_arm64_struct_loop:\n");
    out.push_str("    cmp x22, x21\n");
    out.push_str("    b.ge .L_arm64_struct_close\n");
    out.push_str("    cbz x22, .L_arm64_struct_print_f\n");
    emit_adrp_add(out, "x0", "alya_fmt_struct_comma", os);
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str(".L_arm64_struct_print_f:\n");
    emit_adrp_add(out, "x0", "alya_fmt_struct_field", os);
    out.push_str("    add x23, x22, #2\n");
    out.push_str("    ldr x1, [x20, x23, lsl #3]\n");
    out.push_str("    add x23, x22, #1\n");
    out.push_str("    ldr x2, [x19, x23, lsl #3]\n");
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        out.push_str("    str x2, [sp, #8]\n");
        out.push_str(&format!("    bl {}printf\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}printf\n", p));
    }
    out.push_str("    add x22, x22, #1\n");
    out.push_str("    b .L_arm64_struct_loop\n");
    out.push_str(".L_arm64_struct_close:\n");
    emit_adrp_add(out, "x0", "alya_fmt_struct_close", os);
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str(".L_arm64_struct_exit:\n");
    out.push_str("    ldr x23, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #64\n");
    out.push_str("    ret\n\n");

}
