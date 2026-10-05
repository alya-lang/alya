use crate::codegen::kinds::KIND_STRING;
use crate::codegen::target::OperatingSystem;
use super::{emit_adrp_add, emit_str_buf_ctx};

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_contains
    out.push_str(".align 2\n");
    out.push_str("fn_contains:\n");
    out.push_str("    ldrb w2, [x1]\n");
    out.push_str("    cbz w2, .L_arm64_contains_match\n");
    out.push_str(".L_arm64_contains_outer:\n");
    out.push_str("    ldrb w2, [x0]\n");
    out.push_str("    cbz w2, .L_arm64_contains_nomatch\n");
    out.push_str("    mov x3, x0\n");
    out.push_str("    mov x4, x1\n");
    out.push_str(".L_arm64_contains_inner:\n");
    out.push_str("    ldrb w5, [x4]\n");
    out.push_str("    cbz w5, .L_arm64_contains_match\n");
    out.push_str("    ldrb w6, [x3]\n");
    out.push_str("    cmp w6, w5\n");
    out.push_str("    b.ne .L_arm64_contains_next\n");
    out.push_str("    add x3, x3, #1\n");
    out.push_str("    add x4, x4, #1\n");
    out.push_str("    b .L_arm64_contains_inner\n");
    out.push_str(".L_arm64_contains_next:\n");
    out.push_str("    add x0, x0, #1\n");
    out.push_str("    b .L_arm64_contains_outer\n");
    out.push_str(".L_arm64_contains_match:\n");
    out.push_str("    mov x0, #1\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_contains_nomatch:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_join
    out.push_str(".global fn_join\n");
    out.push_str("fn_join:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    stp x25, x26, [sp, #64]\n");
    out.push_str("    mov x19, x0\n"); // arr
    out.push_str("    mov x20, x1\n"); // delim
    out.push_str("    cbz x19, .L_arm64_join_empty\n");
    out.push_str("    ldr x21, [x19]\n"); // arr->len
    out.push_str("    cbz x21, .L_arm64_join_empty\n");
    emit_str_buf_ctx(out, "x22", "x23", "x9", os);
    out.push_str("    ldr x24, [x23]\n");
    out.push_str("    movz x9, #16960\n");
    out.push_str("    movk x9, #15, lsl #16\n"); // 1,000,000
    out.push_str("    cmp x24, x9\n");
    out.push_str("    b.lo .L_arm64_join_buf_ok\n");
    out.push_str("    mov x24, #0\n");
    out.push_str(".L_arm64_join_buf_ok:\n");
    out.push_str("    add x25, x22, x24\n"); // write_ptr
    out.push_str("    mov x26, x25\n"); // start_ptr
    out.push_str("    mov x2, #0\n"); // i = 0
    out.push_str(".L_arm64_join_loop:\n");
    out.push_str("    cmp x2, x21\n");
    out.push_str("    b.ge .L_arm64_join_finish\n");
    out.push_str("    cbz x2, .L_arm64_join_copy_elem\n");
    out.push_str("    mov x3, x20\n"); // copy delim
    out.push_str(".L_arm64_join_copy_delim:\n");
    out.push_str("    ldrb w4, [x3], #1\n");
    out.push_str("    cbz w4, .L_arm64_join_copy_elem\n");
    out.push_str("    strb w4, [x25], #1\n");
    out.push_str("    b .L_arm64_join_copy_delim\n");
    out.push_str(".L_arm64_join_copy_elem:\n");
    out.push_str("    ldr x3, [x19, #16]\n"); // arr->data
    out.push_str("    ldr x3, [x3, x2, lsl #3]\n"); // arr->data[i]
    out.push_str(".L_arm64_join_copy_elem_loop:\n");
    out.push_str("    ldrb w4, [x3], #1\n");
    out.push_str("    cbz w4, .L_arm64_join_elem_done\n");
    out.push_str("    strb w4, [x25], #1\n");
    out.push_str("    b .L_arm64_join_copy_elem_loop\n");
    out.push_str(".L_arm64_join_elem_done:\n");
    out.push_str("    add x2, x2, #1\n");
    out.push_str("    b .L_arm64_join_loop\n");
    out.push_str(".L_arm64_join_finish:\n");
    out.push_str("    strb wzr, [x25], #1\n");
    out.push_str("    sub x3, x25, x22\n");
    out.push_str("    add x3, x3, #7\n");
    out.push_str("    and x3, x3, #~7\n");
    out.push_str("    str x3, [x23]\n");
    out.push_str("    mov x0, x26\n");
    out.push_str("    b .L_arm64_join_ret\n");
    out.push_str(".L_arm64_join_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_join_ret:\n");
    out.push_str("    ldp x25, x26, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // fn_split
    out.push_str(".global fn_split\n");
    out.push_str("fn_split:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    stp x25, x26, [sp, #64]\n");
    out.push_str("    mov x19, x0\n"); // str
    out.push_str("    mov x20, x1\n"); // delim
    out.push_str("    mov x0, #0\n");
    out.push_str("    bl alya_array_new\n");
    out.push_str("    mov x21, x0\n"); // arr
                                       // compute delim_len
    out.push_str("    mov x22, #0\n"); // delim_len
    out.push_str(".L_arm64_split_dlen_loop:\n");
    out.push_str("    ldrb w2, [x20, x22]\n");
    out.push_str("    cbz w2, .L_arm64_split_dlen_done\n");
    out.push_str("    add x22, x22, #1\n");
    out.push_str("    b .L_arm64_split_dlen_loop\n");
    out.push_str(".L_arm64_split_dlen_done:\n");
    out.push_str("    cbnz x22, .L_arm64_split_non_empty_delim\n");
    // empty delim: split char by char
    out.push_str("    mov x23, x19\n");
    out.push_str(".L_arm64_split_empty_loop:\n");
    out.push_str("    ldrb w24, [x23], #1\n");
    out.push_str("    cbz w24, .L_arm64_split_ret\n");
    emit_str_buf_ctx(out, "x9", "x10", "x12", os);
    out.push_str("    ldr x11, [x10]\n");
    out.push_str("    movz x12, #16960\n");
    out.push_str("    movk x12, #15, lsl #16\n"); // 1,000,000
    out.push_str("    cmp x11, x12\n");
    out.push_str("    b.lo .L_arm64_se1\n");
    out.push_str("    mov x11, #0\n");
    out.push_str(".L_arm64_se1:\n");
    out.push_str("    add x12, x9, x11\n"); // token ptr
    out.push_str("    strb w24, [x12]\n");
    out.push_str("    strb wzr, [x12, #1]\n");
    out.push_str("    add x13, x11, #2\n");
    out.push_str("    add x13, x13, #7\n");
    out.push_str("    and x13, x13, #~7\n");
    out.push_str("    str x13, [x10]\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    mov x1, x12\n");
    out.push_str(&format!("    mov x2, #{}\n", KIND_STRING));
    out.push_str("    bl alya_array_push\n");
    out.push_str("    b .L_arm64_split_empty_loop\n");
    // non-empty delim
    out.push_str(".L_arm64_split_non_empty_delim:\n");
    out.push_str("    mov x23, x19\n"); // curr
    out.push_str("    mov x24, x19\n"); // token_start
    out.push_str(".L_arm64_split_main_loop:\n");
    out.push_str("    ldrb w9, [x23]\n");
    out.push_str("    cbz w9, .L_arm64_split_emit_final\n");
    out.push_str("    mov x25, #0\n"); // k = 0
    out.push_str(".L_arm64_split_cmp_loop:\n");
    out.push_str("    cmp x25, x22\n");
    out.push_str("    b.ge .L_arm64_split_matched\n");
    out.push_str("    ldrb w10, [x20, x25]\n");
    out.push_str("    ldrb w11, [x23, x25]\n");
    out.push_str("    cmp w10, w11\n");
    out.push_str("    b.ne .L_arm64_split_cmp_fail\n");
    out.push_str("    add x25, x25, #1\n");
    out.push_str("    b .L_arm64_split_cmp_loop\n");
    out.push_str(".L_arm64_split_cmp_fail:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_split_main_loop\n");
    out.push_str(".L_arm64_split_matched:\n");
    emit_str_buf_ctx(out, "x9", "x10", "x12", os);
    out.push_str("    ldr x11, [x10]\n");
    out.push_str("    movz x12, #16960\n");
    out.push_str("    movk x12, #15, lsl #16\n"); // 1,000,000
    out.push_str("    cmp x11, x12\n");
    out.push_str("    b.lo .L_arm64_se2\n");
    out.push_str("    mov x11, #0\n");
    out.push_str(".L_arm64_se2:\n");
    out.push_str("    add x25, x9, x11\n"); // token ptr
    out.push_str("    mov x26, x25\n"); // write ptr
    out.push_str("    mov x2, x24\n"); // src = token_start
    out.push_str(".L_arm64_split_tok_copy:\n");
    out.push_str("    cmp x2, x23\n");
    out.push_str("    b.ge .L_arm64_split_tok_done\n");
    out.push_str("    ldrb w3, [x2], #1\n");
    out.push_str("    strb w3, [x26], #1\n");
    out.push_str("    b .L_arm64_split_tok_copy\n");
    out.push_str(".L_arm64_split_tok_done:\n");
    // Zero-length tokens canonicalize to the rodata empty (#92).
    // The cursor commit below still runs (a few idle bytes).
    out.push_str("    cmp x24, x23\n");
    out.push_str("    b.ne .L_arm64_split_tok_nonempty\n");
    emit_adrp_add(out, "x25", "alya_str_empty", os);
    out.push_str(".L_arm64_split_tok_nonempty:\n");
    out.push_str("    strb wzr, [x26], #1\n");
    out.push_str("    sub x13, x26, x9\n");
    out.push_str("    add x13, x13, #7\n");
    out.push_str("    and x13, x13, #~7\n");
    out.push_str("    str x13, [x10]\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    mov x1, x25\n");
    out.push_str(&format!("    mov x2, #{}\n", KIND_STRING));
    out.push_str("    bl alya_array_push\n");
    out.push_str("    add x23, x23, x22\n"); // curr += delim_len
    out.push_str("    mov x24, x23\n"); // token_start = curr
    out.push_str("    b .L_arm64_split_main_loop\n");
    out.push_str(".L_arm64_split_emit_final:\n");
    emit_str_buf_ctx(out, "x9", "x10", "x12", os);
    out.push_str("    ldr x11, [x10]\n");
    out.push_str("    movz x12, #16960\n");
    out.push_str("    movk x12, #15, lsl #16\n"); // 1,000,000
    out.push_str("    cmp x11, x12\n");
    out.push_str("    b.lo .L_arm64_se3\n");
    out.push_str("    mov x11, #0\n");
    out.push_str(".L_arm64_se3:\n");
    out.push_str("    add x25, x9, x11\n"); // token ptr
    out.push_str("    mov x26, x25\n");
    out.push_str("    mov x2, x24\n"); // token_start
    out.push_str(".L_arm64_split_final_copy:\n");
    out.push_str("    cmp x2, x23\n");
    out.push_str("    b.ge .L_arm64_split_final_done\n");
    out.push_str("    ldrb w3, [x2], #1\n");
    out.push_str("    strb w3, [x26], #1\n");
    out.push_str("    b .L_arm64_split_final_copy\n");
    out.push_str(".L_arm64_split_final_done:\n");
    // Same canonicalization for the trailing piece (#92).
    out.push_str("    cmp x24, x23\n");
    out.push_str("    b.ne .L_arm64_split_final_nonempty\n");
    emit_adrp_add(out, "x25", "alya_str_empty", os);
    out.push_str(".L_arm64_split_final_nonempty:\n");
    out.push_str("    strb wzr, [x26], #1\n");
    out.push_str("    sub x13, x26, x9\n");
    out.push_str("    add x13, x13, #7\n");
    out.push_str("    and x13, x13, #~7\n");
    out.push_str("    str x13, [x10]\n");
    out.push_str("    mov x0, x21\n");
    out.push_str("    mov x1, x25\n");
    out.push_str(&format!("    mov x2, #{}\n", KIND_STRING));
    out.push_str("    bl alya_array_push\n");
    out.push_str(".L_arm64_split_ret:\n");
    out.push_str("    mov x0, x21\n"); // return arr
    out.push_str("    ldp x25, x26, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

}
