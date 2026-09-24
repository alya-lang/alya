use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_alloc
    out.push_str(".align 2\n");
    out.push_str(".global fn_alloc\n");
    out.push_str("fn_alloc:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");
    emit_adrp_add(out, "x1", "alya_allocated_bytes", os);
    out.push_str("    ldr x2, [x1]\n");
    out.push_str("    add x2, x2, x0\n");
    out.push_str("    str x2, [x1]\n");
    out.push_str("    ldr x0, [sp, #16]\n");
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    str x0, [sp, #24]\n");
    out.push_str("    ldr x1, [sp, #16]\n");
    out.push_str("    mov x2, #5\n");
    out.push_str("    mov x3, #0\n");
    out.push_str("    bl alya_mem_track_alloc\n");
    out.push_str("    ldr x0, [sp, #24]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_free
    out.push_str(".align 2\n");
    out.push_str(".global fn_free\n");
    out.push_str("fn_free:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_free_done\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    bl alya_mem_track_free\n");
    out.push_str("    ldr x0, [sp, #16]\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str(".L_arm64_free_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_realloc
    out.push_str(".align 2\n");
    out.push_str(".global fn_realloc\n");
    out.push_str("fn_realloc:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    str x1, [sp, #24]\n");
    emit_adrp_add(out, "x2", "alya_allocated_bytes", os);
    out.push_str("    ldr x3, [x2]\n");
    out.push_str("    add x3, x3, x1\n");
    out.push_str("    str x3, [x2]\n");
    out.push_str("    ldr x0, [sp, #16]\n");
    out.push_str("    ldr x1, [sp, #24]\n");
    out.push_str(&format!("    bl {}realloc\n", p));
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_copy_mem
    out.push_str(".align 2\n");
    out.push_str(".global fn_copy_mem\n");
    out.push_str("fn_copy_mem:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(&format!("    bl {}memcpy\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_zero_mem
    out.push_str(".align 2\n");
    out.push_str(".global fn_zero_mem\n");
    out.push_str("fn_zero_mem:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x2, x1\n");
    out.push_str("    mov x1, #0\n");
    out.push_str(&format!("    bl {}memset\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_peek_byte
    out.push_str(".align 2\n");
    out.push_str(".global fn_peek_byte\n");
    out.push_str("fn_peek_byte:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldrb w0, [x0]\n");
    out.push_str("    ret\n\n");

    // fn_poke_byte
    out.push_str(".align 2\n");
    out.push_str(".global fn_poke_byte\n");
    out.push_str("fn_poke_byte:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    strb w2, [x0]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_peek_int
    out.push_str(".align 2\n");
    out.push_str(".global fn_peek_int\n");
    out.push_str("fn_peek_int:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldr x0, [x0]\n");
    out.push_str("    ret\n\n");

    // fn_poke_int
    out.push_str(".align 2\n");
    out.push_str(".global fn_poke_int\n");
    out.push_str("fn_poke_int:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    str x2, [x0]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_mem_peek_f32(x0=ptr, x1=offset) -> float (f64 bits in x0)
    // Loads a 32-bit float and widens it to f64 (exact widening).
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_peek_f32\n");
    out.push_str("fn_mem_peek_f32:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldr s0, [x0]\n");
    out.push_str("    fcvt d0, s0\n");
    out.push_str("    fmov x0, d0\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_f32(x0=ptr, x1=offset, x2=f64 bits) -> void
    // Narrows with hardware round-to-nearest-even (fcvt s0, d0).
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_poke_f32\n");
    out.push_str("fn_mem_poke_f32:\n");
    out.push_str("    fmov d0, x2\n");
    out.push_str("    fcvt s0, d0\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    str s0, [x0]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_mem_peek_i32(x0=ptr, x1=offset) -> int (sign-extended)
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_peek_i32\n");
    out.push_str("fn_mem_peek_i32:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldrsw x0, [x0]\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_i32(x0=ptr, x1=offset, x2=val) -> void (low 32 bits)
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_poke_i32\n");
    out.push_str("fn_mem_poke_i32:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    str w2, [x0]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_str_from_ptr
    out.push_str(".align 2\n");
    out.push_str(".global fn_str_from_ptr\n");
    out.push_str("fn_str_from_ptr:\n");
    out.push_str("    cbnz x0, .L_arm64_sfp_ret\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_sfp_ret:\n");
    out.push_str("    ret\n\n");

    // fn_str_to_ptr
    out.push_str(".align 2\n");
    out.push_str(".global fn_str_to_ptr\n");
    out.push_str("fn_str_to_ptr:\n");
    out.push_str("    ret\n\n");

    // fn_mem_allocated
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_allocated\n");
    out.push_str("fn_mem_allocated:\n");
    emit_adrp_add(out, "x1", "alya_allocated_bytes", os);
    out.push_str("    ldr x0, [x1]\n");
    out.push_str("    ret\n\n");

    // fn_mem_reset_alloc
    out.push_str(".align 2\n");
    out.push_str(".global fn_mem_reset_alloc\n");
    out.push_str("fn_mem_reset_alloc:\n");
    emit_adrp_add(out, "x1", "alya_allocated_bytes", os);
    out.push_str("    str xzr, [x1]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_str_clone
    out.push_str(".align 2\n");
    out.push_str(".global fn_str_clone\n");
    out.push_str("fn_str_clone:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    str x21, [sp, #32]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    cbz x19, .L_arm64_sclone_empty\n");
    out.push_str("    mov x20, x19\n");
    out.push_str("    mov x21, #0\n");
    out.push_str(".L_arm64_sclone_len:\n");
    out.push_str("    ldrb w1, [x20]\n");
    out.push_str("    cbz w1, .L_arm64_sclone_alloc\n");
    out.push_str("    add x20, x20, #1\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    b .L_arm64_sclone_len\n");
    out.push_str(".L_arm64_sclone_alloc:\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    mov x0, x21\n");
    emit_adrp_add(out, "x1", "alya_allocated_bytes", os);
    out.push_str("    ldr x2, [x1]\n");
    out.push_str("    add x2, x2, x21\n");
    out.push_str("    str x2, [x1]\n");
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    cbz x0, .L_arm64_sclone_empty\n");
    out.push_str("    mov x20, x0\n");
    out.push_str("    mov x1, x19\n");
    out.push_str("    mov x2, x21\n");
    out.push_str(&format!("    bl {}memcpy\n", p));
    out.push_str("    mov x0, x20\n");
    out.push_str("    mov x1, x21\n");
    out.push_str("    mov x2, #4\n"); // kind: String
    out.push_str("    mov x3, #0\n");
    out.push_str("    bl alya_mem_track_alloc\n");
    out.push_str("    mov x0, x20\n");
    out.push_str("    b .L_arm64_sclone_done\n");
    out.push_str(".L_arm64_sclone_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_sclone_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldr x21, [sp, #32]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn_str_free
    out.push_str(".align 2\n");
    out.push_str(".global fn_str_free\n");
    out.push_str("fn_str_free:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_sfree_done\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    bl alya_mem_track_free\n");
    out.push_str("    ldr x0, [sp, #16]\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str(".L_arm64_sfree_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_rc_retain
    out.push_str(".align 2\n");
    out.push_str(".global fn_rc_retain\n");
    out.push_str("fn_rc_retain:\n");
    out.push_str("    tst x0, #7\n");
    out.push_str("    b.ne .L_arm64_rc_retain_done\n");
    out.push_str("    cmp x0, #65536\n");
    out.push_str("    b.ls .L_arm64_rc_retain_done\n");
    out.push_str("    lsr x1, x0, #47\n");
    out.push_str("    cbnz x1, .L_arm64_rc_retain_done\n");
    emit_adrp_add(out, "x1", "alya_rodata_start", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_ret_chk_str_buf\n");
    emit_adrp_add(out, "x2", "alya_rodata_end", os);
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    out.push_str(".L_arm64_rc_ret_chk_str_buf:\n");
    emit_adrp_add(out, "x1", "alya_str_buf", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_ret_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    out.push_str(".L_arm64_rc_ret_chk_tag:\n");
    out.push_str("    ldur x1, [x0, #-16]\n");
    out.push_str("    uxtw x1, w1\n");
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rc_retain_ok\n");
    out.push_str("    movz x2, #0x0002\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rc_retain_ok\n");
    out.push_str("    movz x2, #0x0003\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rc_retain_ok\n");
    out.push_str("    movz x2, #0x0004\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.ne .L_arm64_rc_retain_done\n");
    out.push_str(".L_arm64_rc_retain_ok:\n");
    out.push_str("    sub x3, x0, #8\n");
    out.push_str(".L_arm64_rc_retain_loop:\n");
    out.push_str("    ldxr x4, [x3]\n");
    out.push_str("    add x4, x4, #1\n");
    out.push_str("    stxr w5, x4, [x3]\n");
    out.push_str("    cbnz w5, .L_arm64_rc_retain_loop\n");
    out.push_str("    stur wzr, [x0, #-12]\n");
    out.push_str(".L_arm64_rc_retain_done:\n");
    out.push_str("    ret\n\n");

    // fn_rc_release
    out.push_str(".align 2\n");
    out.push_str(".global fn_rc_release\n");
    out.push_str("fn_rc_release:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_rc_rel_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.ls .L_arm64_rc_rel_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_rc_rel_done\n");
    emit_adrp_add(out, "x1", "alya_rodata_start", os);
    out.push_str("    cmp x19, x1\n");
    out.push_str("    b.lo .L_arm64_rc_rel_chk_str_buf\n");
    emit_adrp_add(out, "x2", "alya_rodata_end", os);
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_chk_str_buf:\n");
    emit_adrp_add(out, "x1", "alya_str_buf", os);
    out.push_str("    cmp x19, x1\n");
    out.push_str("    b.lo .L_arm64_rc_rel_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_chk_tag:\n");
    out.push_str("    ldur x20, [x19, #-16]\n");
    out.push_str("    uxtw x20, w20\n");
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_rel_ok\n");
    out.push_str("    movz x2, #0x0002\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_rel_ok\n");
    out.push_str("    movz x2, #0x0003\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_rel_ok\n");
    out.push_str("    movz x2, #0x0004\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.ne .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_ok:\n");
    out.push_str("    sub x3, x19, #8\n");
    out.push_str(".L_arm64_rc_rel_loop:\n");
    out.push_str("    ldxr x4, [x3]\n");
    out.push_str("    sub x4, x4, #1\n");
    out.push_str("    stxr w5, x4, [x3]\n");
    out.push_str("    cbnz w5, .L_arm64_rc_rel_loop\n");
    out.push_str("    cbnz x4, .L_arm64_rc_rel_purple\n");
    out.push_str("    stur wzr, [x19, #-12]\n");
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_free_inner\n");
    out.push_str("    movz x2, #0x0002\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_free_inner\n");
    out.push_str("    b .L_arm64_rc_free_outer\n");
    out.push_str(".L_arm64_rc_free_inner:\n");
    out.push_str("    ldr x0, [x19, #16]\n");
    out.push_str("    cbz x0, .L_arm64_rc_free_outer\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str(".L_arm64_rc_free_outer:\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    bl alya_mem_track_free\n");
    out.push_str("    sub x0, x19, #16\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str("    b .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_purple:\n");
    out.push_str("    movz x2, #0x0004\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.eq .L_arm64_rc_rel_done\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    bl fn_gc_add_purple\n");
    out.push_str(".L_arm64_rc_rel_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn_rc_count
    out.push_str(".align 2\n");
    out.push_str(".global fn_rc_count\n");
    out.push_str("fn_rc_count:\n");
    out.push_str("    tst x0, #7\n");
    out.push_str("    b.ne .L_arm64_rcc_zero\n");
    out.push_str("    cmp x0, #65536\n");
    out.push_str("    b.ls .L_arm64_rcc_zero\n");
    out.push_str("    lsr x1, x0, #47\n");
    out.push_str("    cbnz x1, .L_arm64_rcc_zero\n");
    emit_adrp_add(out, "x1", "alya_rodata_start", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rcc_chk_str_buf\n");
    emit_adrp_add(out, "x2", "alya_rodata_end", os);
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rcc_zero\n");
    out.push_str(".L_arm64_rcc_chk_str_buf:\n");
    emit_adrp_add(out, "x1", "alya_str_buf", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rcc_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rcc_zero\n");
    out.push_str(".L_arm64_rcc_chk_tag:\n");
    out.push_str("    ldur x1, [x0, #-16]\n");
    out.push_str("    uxtw x1, w1\n");
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rcc_ok\n");
    out.push_str("    movz x2, #0x0002\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rcc_ok\n");
    out.push_str("    movz x2, #0x0003\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rcc_ok\n");
    out.push_str("    movz x2, #0x0004\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.eq .L_arm64_rcc_ok\n");
    out.push_str(".L_arm64_rcc_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_rcc_ok:\n");
    out.push_str("    ldur x0, [x0, #-8]\n");
    out.push_str("    ret\n\n");
}
