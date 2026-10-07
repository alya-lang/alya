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

    // alya_mem_header_readable(value): 1 if the 16 header bytes at
    // value-16 are fully committed+readable, else 0. Never faults.
    // Windows-only (alya-lang/alya#117): IsBadReadPtr cannot be used
    // here because its internal probe fault is stolen by our own VEH
    // crash handler, killing the process instead of returning nonzero.
    // VirtualQuery is a pure query with no fault path.
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(".align 2\n");
        out.push_str(".global alya_mem_header_readable\n");
        out.push_str("alya_mem_header_readable:\n");
        out.push_str("    stp x29, x30, [sp, #-112]!\n");
        out.push_str("    mov x29, sp\n");
        // 112 bytes: 32 shadow + 48 struct + 32 save.
        out.push_str("    str x0, [sp, #80]\n");
        out.push_str("    sub x0, x0, #16\n");
        out.push_str("    mov x1, sp\n");
        out.push_str("    add x1, x1, #32\n");
        out.push_str("    mov x2, #48\n");
        out.push_str("    bl VirtualQuery\n");
        out.push_str("    cmp x0, #48\n");
        out.push_str("    b.ne .L_arm64_mhr_no\n");
        // State == MEM_COMMIT (0x1000)?
        out.push_str("    ldr w0, [sp, #64]\n");
        out.push_str("    cmp w0, #0x1000\n");
        out.push_str("    b.ne .L_arm64_mhr_no\n");
        // Base <= hdr?
        out.push_str("    ldr x0, [sp, #32]\n");
        out.push_str("    ldr x1, [sp, #80]\n");
        out.push_str("    sub x1, x1, #16\n");
        out.push_str("    cmp x1, x0\n");
        out.push_str("    b.lo .L_arm64_mhr_no\n");
        // hdr+16 <= Base+Size?
        out.push_str("    ldr x0, [sp, #56]\n");
        out.push_str("    ldr x1, [sp, #32]\n");
        out.push_str("    add x0, x0, x1\n");
        out.push_str("    ldr x1, [sp, #80]\n");
        out.push_str("    cmp x1, x0\n");
        out.push_str("    b.hi .L_arm64_mhr_no\n");
        out.push_str("    mov x0, #1\n");
        out.push_str("    b .L_arm64_mhr_done\n");
        out.push_str(".L_arm64_mhr_no:\n");
        out.push_str("    mov x0, #0\n");
        out.push_str(".L_arm64_mhr_done:\n");
        out.push_str("    ldp x29, x30, [sp], #112\n");
        out.push_str("    ret\n\n");
    }

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
    // Stable strings are immortal and never refcounted; skipping also
    // avoids gambling the header read on whatever precedes the region.
    emit_adrp_add(out, "x1", "alya_str_stable", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_ret_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    out.push_str(".L_arm64_rc_ret_chk_tag:\n");
    // Readability probe (alya-lang/alya#117): raw big ints in unmapped
    // gaps fault on the header read below. Verify the 16 header bytes
    // are mapped first; unmapped -> skip (safe direction: at worst a
    // leak, never a fault). Fires only for real heap objects and
    // high-gap ints. Saves value and lr together (otherwise call-free).
    if matches!(os, OperatingSystem::Windows) {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        // Save value + lr together (otherwise a call-free leaf).
        out.push_str("    stp x0, x30, [sp, #-32]!\n");
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    ldr x0, [sp, #32]\n");
        out.push_str("    bl alya_mem_header_readable\n");
        out.push_str("    add sp, sp, #32\n");
        out.push_str("    mov w9, w0\n");
        out.push_str("    ldp x0, x30, [sp], #32\n");
        out.push_str("    cbz w9, .L_arm64_rc_retain_done\n");
    } else {
        // msync returns 0 when mapped, ENOMEM otherwise. Aligned base
        // + exact span avoids over-probing into neighbors.
        out.push_str("    stp x0, x30, [sp, #-32]!\n");
        out.push_str("    sub x0, x0, #16\n");
        if matches!(os, OperatingSystem::MacOS) {
            // Apple Silicon pages are 16K: a 4K-aligned base is not a
            // page multiple and msync rejects it with EINVAL.
            out.push_str("    lsr x0, x0, #14\n");
            out.push_str("    lsl x0, x0, #14\n");
        } else {
            out.push_str("    and x0, x0, #-4096\n");
        }
        out.push_str("    ldr x1, [sp]\n");
        out.push_str("    sub x1, x1, x0\n");
        out.push_str("    mov x2, #1\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    bl _msync\n");
        } else {
            out.push_str("    bl msync\n");
        }
        // Save the result before restoring: ldp would overwrite w0
        // with the value and the test would read the pointer.
        out.push_str("    mov w9, w0\n");
        out.push_str("    ldp x0, x30, [sp], #32\n");
        out.push_str("    cbnz w9, .L_arm64_rc_retain_done\n");
    }
    // Shared tag-read entry for fn_rc_retain_direct (same registers and
    // frame shape; see the x64 counterpart).
    out.push_str(".L_arm64_rc_ret_tag_read:\n");
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

    // fn_rc_retain_direct: probe-free fast path for statically-proven
    // heap values (see the x64 counterpart). Leaf like the probed form.
    out.push_str(".align 2\n");
    out.push_str(".global fn_rc_retain_direct\n");
    out.push_str("fn_rc_retain_direct:\n");
    out.push_str("    tst x0, #7\n");
    out.push_str("    b.ne .L_arm64_rc_retain_done\n");
    out.push_str("    cmp x0, #65536\n");
    out.push_str("    b.ls .L_arm64_rc_retain_done\n");
    out.push_str("    lsr x1, x0, #47\n");
    out.push_str("    cbnz x1, .L_arm64_rc_retain_done\n");
    emit_adrp_add(out, "x1", "alya_rodata_start", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_retain_direct_chk_str_buf\n");
    emit_adrp_add(out, "x2", "alya_rodata_end", os);
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    out.push_str(".L_arm64_rc_retain_direct_chk_str_buf:\n");
    emit_adrp_add(out, "x1", "alya_str_buf", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_ret_tag_read\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    emit_adrp_add(out, "x1", "alya_str_stable", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rc_ret_tag_read\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rc_retain_done\n");
    out.push_str("    b .L_arm64_rc_ret_tag_read\n\n");

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
    // Stable strings are immortal and never refcounted.
    emit_adrp_add(out, "x1", "alya_str_stable", os);
    out.push_str("    cmp x19, x1\n");
    out.push_str("    b.lo .L_arm64_rc_rel_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_chk_tag:\n");
    // Readability probe, same contract as retain (alya-lang/alya#117).
    // x19 is callee-saved and survives the call; no spill needed.
    if matches!(os, OperatingSystem::Windows) {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        // x19 is callee-saved and survives the call; no spill needed.
        // The 32-byte sub keeps the call 16-aligned and reserves the
        // Windows home area.
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    mov x0, x19\n");
        out.push_str("    bl alya_mem_header_readable\n");
        out.push_str("    add sp, sp, #32\n");
        out.push_str("    cbz w0, .L_arm64_rc_rel_done\n");
    } else {
        out.push_str("    sub x0, x19, #16\n");
        if matches!(os, OperatingSystem::MacOS) {
            // Apple Silicon pages are 16K (see retain).
            out.push_str("    lsr x0, x0, #14\n");
            out.push_str("    lsl x0, x0, #14\n");
        } else {
            out.push_str("    and x0, x0, #-4096\n");
        }
        out.push_str("    sub x1, x19, x0\n");
        out.push_str("    mov x2, #1\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    bl _msync\n");
        } else {
            out.push_str("    bl msync\n");
        }
        out.push_str("    cbnz w0, .L_arm64_rc_rel_done\n");
    }
    // Shared tag-read entry for fn_rc_release_direct (x19 holds the
    // value in both forms; frames match).
    out.push_str(".L_arm64_rc_rel_tag_read:\n");
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
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.ne .L_arm64_rc_free_inner_map\n");
    // Arrays: release heap-kind elements (kinds 4/5/6 in the kind
    // sidecar) before freeing the buffers, or every element leaks one
    // reference (alya-lang/alya#79). Only statically-known heap kinds
    // are touched. Maps keep the legacy path (entries only).
    out.push_str("    ldr x9, [x19]\n");
    out.push_str("    ldr x10, [x19, #16]\n");
    out.push_str("    ldr x11, [x19, #24]\n");
    out.push_str("    cbz x10, .L_arm64_rc_free_elem\n");
    out.push_str("    cbz x11, .L_arm64_rc_free_elem\n");
    out.push_str("    sub sp, sp, #32\n");
    out.push_str("    str x9, [sp]\n");
    out.push_str("    str x10, [sp, #8]\n");
    out.push_str("    str x11, [sp, #16]\n");
    out.push_str("    mov x12, #0\n");
    out.push_str("    str x12, [sp, #24]\n");
    out.push_str(".L_arm64_rc_cascade_loop:\n");
    out.push_str("    ldr x12, [sp, #24]\n");
    out.push_str("    ldr x9, [sp]\n");
    out.push_str("    cmp x12, x9\n");
    out.push_str("    b.ge .L_arm64_rc_cascade_done\n");
    out.push_str("    ldr x11, [sp, #16]\n");
    out.push_str("    ldrb w13, [x11, x12]\n");
    out.push_str("    cmp w13, #4\n");
    out.push_str("    b.eq .L_arm64_rc_cascade_rel\n");
    out.push_str("    cmp w13, #5\n");
    out.push_str("    b.eq .L_arm64_rc_cascade_rel\n");
    out.push_str("    cmp w13, #6\n");
    out.push_str("    b.ne .L_arm64_rc_cascade_next\n");
    out.push_str(".L_arm64_rc_cascade_rel:\n");
    out.push_str("    ldr x10, [sp, #8]\n");
    out.push_str("    lsl x13, x12, #3\n");
    out.push_str("    ldr x0, [x10, x13]\n");
    out.push_str("    str x12, [sp, #24]\n");
    out.push_str("    bl fn_rc_release\n");
    out.push_str("    ldr x12, [sp, #24]\n");
    out.push_str(".L_arm64_rc_cascade_next:\n");
    out.push_str("    add x12, x12, #1\n");
    out.push_str("    str x12, [sp, #24]\n");
    out.push_str("    b .L_arm64_rc_cascade_loop\n");
    out.push_str(".L_arm64_rc_cascade_done:\n");
    out.push_str("    add sp, sp, #32\n");
    out.push_str(".L_arm64_rc_free_elem:\n");
    out.push_str("    ldr x0, [x19, #16]\n");
    out.push_str("    cbz x0, .L_arm64_rc_free_kind\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str("    b .L_arm64_rc_free_kind\n");
    out.push_str(".L_arm64_rc_free_inner_map:\n");
    out.push_str("    ldr x0, [x19, #16]\n");
    out.push_str("    cbz x0, .L_arm64_rc_free_kind\n");
    // Maps: release heap-kind values (tags 4..6: array, map, struct)
    // before freeing the entries buffer (alya-lang/alya#81). Strings
    // (tag 3) and keys are deliberately untouched: strings carry no
    // refcount header, so releasing them would corrupt the heap.
    out.push_str("    stp x21, x22, [sp, #-32]!\n");
    out.push_str("    stp x23, x24, [sp, #16]\n");
    out.push_str("    ldr x21, [x19, #8]\n");
    out.push_str("    ldr x22, [x19, #16]\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_rc_map_cascade_loop:\n");
    out.push_str("    cmp x23, x21\n");
    out.push_str("    b.ge .L_arm64_rc_map_cascade_done\n");
    out.push_str("    add x24, x23, x23, lsl #1\n");
    out.push_str("    add x24, x22, x24, lsl #3\n");
    out.push_str("    ldr w9, [x24, #16]\n");
    out.push_str("    cmp w9, #1\n");
    out.push_str("    b.ne .L_arm64_rc_map_cascade_next\n");
    // Release value if tag in 4..6 (array, map, struct)
    out.push_str("    ldr w9, [x24, #20]\n");
    out.push_str("    cmp w9, #4\n");
    out.push_str("    b.lt .L_arm64_rc_map_cascade_next\n");
    out.push_str("    cmp w9, #6\n");
    out.push_str("    b.gt .L_arm64_rc_map_cascade_next\n");
    out.push_str("    ldr x0, [x24, #8]\n");
    out.push_str("    bl fn_rc_release\n");
    out.push_str(".L_arm64_rc_map_cascade_next:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_rc_map_cascade_loop\n");
    out.push_str(".L_arm64_rc_map_cascade_done:\n");
    out.push_str("    ldp x23, x24, [sp, #16]\n");
    out.push_str("    ldp x21, x22, [sp], #32\n");
    out.push_str("    ldr x0, [x19, #16]\n");
    out.push_str("    cbz x0, .L_arm64_rc_free_kind\n");
    out.push_str(&format!("    bl {}free\n", p));
    // Array kind sidecar (Phase 1, alya-lang/alya#39): freed with the
    // element buffer. Only arrays carry one (x20 still holds the
    // header magic here); maps share this path but skip along.
    out.push_str(".L_arm64_rc_free_kind:\n");
    out.push_str("    movz x2, #0x0001\n");
    out.push_str("    movk x2, #0x5A11, lsl #16\n");
    out.push_str("    cmp x20, x2\n");
    out.push_str("    b.ne .L_arm64_rc_free_outer\n");
    out.push_str("    ldr x0, [x19, #24]\n");
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

    // fn_rc_release_direct: probe-free fast path for statically-proven
    // heap values (see the x64 counterpart). Same frame as the probed
    // form so the shared tail serves both; its cascade keeps probed
    // recursion.
    out.push_str(".align 2\n");
    out.push_str(".global fn_rc_release_direct\n");
    out.push_str("fn_rc_release_direct:\n");
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
    out.push_str("    b.lo .L_arm64_rc_rel_direct_chk_str_buf\n");
    emit_adrp_add(out, "x2", "alya_rodata_end", os);
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    out.push_str(".L_arm64_rc_rel_direct_chk_str_buf:\n");
    emit_adrp_add(out, "x1", "alya_str_buf", os);
    out.push_str("    cmp x19, x1\n");
    out.push_str("    b.lo .L_arm64_rc_rel_tag_read\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    emit_adrp_add(out, "x1", "alya_str_stable", os);
    out.push_str("    cmp x19, x1\n");
    out.push_str("    b.lo .L_arm64_rc_rel_tag_read\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x19, x2\n");
    out.push_str("    b.lo .L_arm64_rc_rel_done\n");
    out.push_str("    b .L_arm64_rc_rel_tag_read\n\n");

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
    // Stable strings are immortal and never refcounted.
    emit_adrp_add(out, "x1", "alya_str_stable", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.lo .L_arm64_rcc_chk_tag\n");
    out.push_str("    movz x2, #1024, lsl #16\n");
    out.push_str("    add x2, x1, x2\n");
    out.push_str("    cmp x0, x2\n");
    out.push_str("    b.lo .L_arm64_rcc_zero\n");
    out.push_str(".L_arm64_rcc_chk_tag:\n");
    // Readability probe, same contract as retain (alya-lang/alya#117).
    // The result is saved before restoring (ldp would overwrite w0).
    if matches!(os, OperatingSystem::Windows) {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        // The result is saved before restoring (ldp would overwrite w0).
        out.push_str("    stp x0, x30, [sp, #-32]!\n");
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    ldr x0, [sp, #32]\n");
        out.push_str("    bl alya_mem_header_readable\n");
        out.push_str("    add sp, sp, #32\n");
        out.push_str("    mov w9, w0\n");
        out.push_str("    ldp x0, x30, [sp], #32\n");
        out.push_str("    cbz w9, .L_arm64_rcc_zero\n");
    } else {
        out.push_str("    stp x0, x30, [sp, #-32]!\n");
        out.push_str("    sub x0, x0, #16\n");
        if matches!(os, OperatingSystem::MacOS) {
            // Apple Silicon pages are 16K (see retain).
            out.push_str("    lsr x0, x0, #14\n");
            out.push_str("    lsl x0, x0, #14\n");
        } else {
            out.push_str("    and x0, x0, #-4096\n");
        }
        out.push_str("    ldr x1, [sp]\n");
        out.push_str("    sub x1, x1, x0\n");
        out.push_str("    mov x2, #1\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    bl _msync\n");
        } else {
            out.push_str("    bl msync\n");
        }
        out.push_str("    mov w9, w0\n");
        out.push_str("    ldp x0, x30, [sp], #32\n");
        out.push_str("    cbnz w9, .L_arm64_rcc_zero\n");
    }
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
