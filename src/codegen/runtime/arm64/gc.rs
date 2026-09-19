use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // fn_gc_add_purple(ptr: x0) - ARM64
    // Marks candidate container as PURPLE (color 3) and records it in roots set.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_add_purple\n");
    out.push_str("fn_gc_add_purple:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x19, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");

    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_gc_ap_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.lo .L_arm64_gc_ap_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_gc_ap_done\n");

    out.push_str("    ldur w1, [x19, #-16]\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.eq .L_arm64_gc_ap_valid\n");
    out.push_str("    movz w2, #0x0002\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.eq .L_arm64_gc_ap_valid\n");
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.ne .L_arm64_gc_ap_done\n");

    out.push_str(".L_arm64_gc_ap_valid:\n");
    emit_adrp_add(out, "x3", "alya_gc_in_progress", os);
    out.push_str("    ldr x4, [x3]\n");
    out.push_str("    cbnz x4, .L_arm64_gc_ap_done\n");

    out.push_str("    ldur w4, [x19, #-12]\n");
    out.push_str("    cmp w4, #3\n");
    out.push_str("    b.eq .L_arm64_gc_ap_done\n");
    out.push_str("    mov w4, #3\n");
    out.push_str("    stur w4, [x19, #-12]\n");

    emit_adrp_add(out, "x5", "alya_gc_roots_count", os);
    out.push_str("    ldr x6, [x5]\n");
    out.push_str("    cmp x6, #65536\n");
    out.push_str("    b.hs .L_arm64_gc_ap_done\n");

    emit_adrp_add(out, "x7", "alya_gc_roots", os);
    out.push_str("    str x19, [x7, x6, lsl #3]\n");
    out.push_str("    add x6, x6, #1\n");
    out.push_str("    str x6, [x5]\n");
    out.push_str("    cmp x6, #1024\n");
    out.push_str("    b.lo .L_arm64_gc_ap_done\n");

    out.push_str("    bl fn_gc_collect\n");

    out.push_str(".L_arm64_gc_ap_done:\n");
    out.push_str("    ldr x19, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_mark_gray(x0: s) - ARM64
    // Trial-decrements reference counts across cyclic subgraph.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str("alya_gc_mark_gray:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    str x25, [sp, #64]\n");

    out.push_str("    mov x19, x0\n");
    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_mg_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.lo .L_arm64_mg_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_mg_done\n");

    out.push_str("    ldur w20, [x19, #-16]\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.eq .L_arm64_mg_valid\n");
    out.push_str("    movz w2, #0x0002\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.eq .L_arm64_mg_valid\n");
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_mg_done\n");

    out.push_str(".L_arm64_mg_valid:\n");
    out.push_str("    ldur w1, [x19, #-12]\n");
    out.push_str("    cmp w1, #1\n");
    out.push_str("    b.eq .L_arm64_mg_done\n");
    out.push_str("    mov w1, #1\n");
    out.push_str("    stur w1, [x19, #-12]\n"); // Color = GRAY (1)

    // Traverse Struct
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_mg_arr\n");
    out.push_str("    ldr x21, [x19]\n");
    out.push_str("    cbz x21, .L_arm64_mg_done\n");
    out.push_str("    ldr x22, [x21, #8]\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_mg_struct_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_mg_done\n");
    out.push_str("    add x24, x23, #1\n");
    out.push_str("    ldr x25, [x19, x24, lsl #3]\n");
    emit_child_mark_gray(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_mg_struct_loop\n");

    // Traverse Array
    out.push_str(".L_arm64_mg_arr:\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_mg_map\n");
    out.push_str("    ldr x22, [x19]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_mg_done\n");
    out.push_str("    cbz x22, .L_arm64_mg_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_mg_arr_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_mg_done\n");
    out.push_str("    ldr x25, [x21, x23, lsl #3]\n");
    emit_child_mark_gray(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_mg_arr_loop\n");

    // Traverse Map
    out.push_str(".L_arm64_mg_map:\n");
    out.push_str("    ldr x22, [x19, #8]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_mg_done\n");
    out.push_str("    cbz x22, .L_arm64_mg_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_mg_map_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_mg_done\n");
    out.push_str("    add x24, x23, x23, lsl #1\n");
    out.push_str("    add x24, x21, x24, lsl #3\n");
    out.push_str("    ldr x9, [x24, #16]\n");
    out.push_str("    cmp x9, #1\n");
    out.push_str("    b.ne .L_arm64_mg_map_next\n");
    out.push_str("    ldr x25, [x24]\n");
    emit_child_mark_gray(out);
    out.push_str("    ldr x25, [x24, #8]\n");
    emit_child_mark_gray(out);
    out.push_str(".L_arm64_mg_map_next:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_mg_map_loop\n");

    out.push_str(".L_arm64_mg_done:\n");
    out.push_str("    ldr x25, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan(x0: s) - ARM64
    // Classifies objects into WHITE (garbage) or revives to BLACK if rc > 0.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str("alya_gc_scan:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    str x25, [sp, #64]\n");

    out.push_str("    mov x19, x0\n");
    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_s_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.lo .L_arm64_s_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_s_done\n");

    out.push_str("    ldur w1, [x19, #-12]\n");
    out.push_str("    cmp w1, #1\n");
    out.push_str("    b.ne .L_arm64_s_done\n");

    out.push_str("    ldur x1, [x19, #-8]\n");
    out.push_str("    cmp x1, #0\n");
    out.push_str("    b.le .L_arm64_s_to_white\n");

    // External reference alive -> Revive subgraph to BLACK
    out.push_str("    mov x0, x19\n");
    out.push_str("    bl alya_gc_scan_black\n");
    out.push_str("    b .L_arm64_s_done\n");

    out.push_str(".L_arm64_s_to_white:\n");
    out.push_str("    mov w1, #2\n");
    out.push_str("    stur w1, [x19, #-12]\n"); // Color = WHITE (2)
    out.push_str("    ldur w20, [x19, #-16]\n");

    // Struct children scan
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_s_arr\n");
    out.push_str("    ldr x21, [x19]\n");
    out.push_str("    cbz x21, .L_arm64_s_done\n");
    out.push_str("    ldr x22, [x21, #8]\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_s_struct_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_s_done\n");
    out.push_str("    add x24, x23, #1\n");
    out.push_str("    ldr x25, [x19, x24, lsl #3]\n");
    emit_child_scan(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_s_struct_loop\n");

    // Array children scan
    out.push_str(".L_arm64_s_arr:\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_s_map\n");
    out.push_str("    ldr x22, [x19]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_s_done\n");
    out.push_str("    cbz x22, .L_arm64_s_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_s_arr_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_s_done\n");
    out.push_str("    ldr x25, [x21, x23, lsl #3]\n");
    emit_child_scan(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_s_arr_loop\n");

    // Map children scan
    out.push_str(".L_arm64_s_map:\n");
    out.push_str("    ldr x22, [x19, #8]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_s_done\n");
    out.push_str("    cbz x22, .L_arm64_s_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_s_map_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_s_done\n");
    out.push_str("    add x24, x23, x23, lsl #1\n");
    out.push_str("    add x24, x21, x24, lsl #3\n");
    out.push_str("    ldr x9, [x24, #16]\n");
    out.push_str("    cmp x9, #1\n");
    out.push_str("    b.ne .L_arm64_s_map_next\n");
    out.push_str("    ldr x25, [x24]\n");
    emit_child_scan(out);
    out.push_str("    ldr x25, [x24, #8]\n");
    emit_child_scan(out);
    out.push_str(".L_arm64_s_map_next:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_s_map_loop\n");

    out.push_str(".L_arm64_s_done:\n");
    out.push_str("    ldr x25, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan_black(x0: s) - ARM64
    // Restores trial-decremented reference counts for reachable subgraphs.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str("alya_gc_scan_black:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    str x25, [sp, #64]\n");

    out.push_str("    mov x19, x0\n");
    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_sb_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.lo .L_arm64_sb_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_sb_done\n");

    out.push_str("    stur wzr, [x19, #-12]\n"); // Color = BLACK (0)
    out.push_str("    ldur w20, [x19, #-16]\n");

    // Struct children scan black
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_sb_arr\n");
    out.push_str("    ldr x21, [x19]\n");
    out.push_str("    cbz x21, .L_arm64_sb_done\n");
    out.push_str("    ldr x22, [x21, #8]\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_sb_struct_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_sb_done\n");
    out.push_str("    add x24, x23, #1\n");
    out.push_str("    ldr x25, [x19, x24, lsl #3]\n");
    emit_child_scan_black(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_sb_struct_loop\n");

    // Array children scan black
    out.push_str(".L_arm64_sb_arr:\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_sb_map\n");
    out.push_str("    ldr x22, [x19]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_sb_done\n");
    out.push_str("    cbz x22, .L_arm64_sb_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_sb_arr_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_sb_done\n");
    out.push_str("    ldr x25, [x21, x23, lsl #3]\n");
    emit_child_scan_black(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_sb_arr_loop\n");

    // Map children scan black
    out.push_str(".L_arm64_sb_map:\n");
    out.push_str("    ldr x22, [x19, #8]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_sb_done\n");
    out.push_str("    cbz x22, .L_arm64_sb_done\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_sb_map_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_sb_done\n");
    out.push_str("    add x24, x23, x23, lsl #1\n");
    out.push_str("    add x24, x21, x24, lsl #3\n");
    out.push_str("    ldr x9, [x24, #16]\n");
    out.push_str("    cmp x9, #1\n");
    out.push_str("    b.ne .L_arm64_sb_map_next\n");
    out.push_str("    ldr x25, [x24]\n");
    emit_child_scan_black(out);
    out.push_str("    ldr x25, [x24, #8]\n");
    emit_child_scan_black(out);
    out.push_str(".L_arm64_sb_map_next:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_sb_map_loop\n");

    out.push_str(".L_arm64_sb_done:\n");
    out.push_str("    ldr x25, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_collect_white(x0: s) - ARM64
    // Sweeps confirmed WHITE cyclic objects and frees their heap allocations.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str("alya_gc_collect_white:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    str x25, [sp, #64]\n");

    out.push_str("    mov x19, x0\n");
    out.push_str("    tst x19, #7\n");
    out.push_str("    b.ne .L_arm64_cw_done\n");
    out.push_str("    cmp x19, #65536\n");
    out.push_str("    b.lo .L_arm64_cw_done\n");
    out.push_str("    lsr x1, x19, #47\n");
    out.push_str("    cbnz x1, .L_arm64_cw_done\n");

    out.push_str("    ldur w1, [x19, #-12]\n");
    out.push_str("    cmp w1, #2\n"); // Must be WHITE (2)
    out.push_str("    b.ne .L_arm64_cw_done\n");
    out.push_str("    stur wzr, [x19, #-12]\n"); // Mark BLACK (0) to prevent duplicate sweeping
    out.push_str("    ldur w20, [x19, #-16]\n");

    // Struct children collection
    out.push_str("    movz w2, #0x0003\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_cw_arr\n");
    out.push_str("    ldr x21, [x19]\n");
    out.push_str("    cbz x21, .L_arm64_cw_free\n");
    out.push_str("    ldr x22, [x21, #8]\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_cw_struct_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_cw_free\n");
    out.push_str("    add x24, x23, #1\n");
    out.push_str("    ldr x25, [x19, x24, lsl #3]\n");
    emit_child_collect_white(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_cw_struct_loop\n");

    // Array children collection
    out.push_str(".L_arm64_cw_arr:\n");
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_cw_map\n");
    out.push_str("    ldr x22, [x19]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_cw_free\n");
    out.push_str("    cbz x22, .L_arm64_cw_free\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_cw_arr_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_cw_free\n");
    out.push_str("    ldr x25, [x21, x23, lsl #3]\n");
    emit_child_collect_white(out);
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_cw_arr_loop\n");

    // Map children collection
    out.push_str(".L_arm64_cw_map:\n");
    out.push_str("    ldr x22, [x19, #8]\n");
    out.push_str("    ldr x21, [x19, #16]\n");
    out.push_str("    cbz x21, .L_arm64_cw_free\n");
    out.push_str("    cbz x22, .L_arm64_cw_free\n");
    out.push_str("    mov x23, #0\n");
    out.push_str(".L_arm64_cw_map_loop:\n");
    out.push_str("    cmp x23, x22\n");
    out.push_str("    b.ge .L_arm64_cw_free\n");
    out.push_str("    add x24, x23, x23, lsl #1\n");
    out.push_str("    add x24, x21, x24, lsl #3\n");
    out.push_str("    ldr x9, [x24, #16]\n");
    out.push_str("    cmp x9, #1\n");
    out.push_str("    b.ne .L_arm64_cw_map_next\n");
    out.push_str("    ldr x25, [x24]\n");
    emit_child_collect_white(out);
    out.push_str("    ldr x25, [x24, #8]\n");
    emit_child_collect_white(out);
    out.push_str(".L_arm64_cw_map_next:\n");
    out.push_str("    add x23, x23, #1\n");
    out.push_str("    b .L_arm64_cw_map_loop\n");

    out.push_str(".L_arm64_cw_free:\n");
    // Free inner data buffers for Array and Map
    out.push_str("    movz w2, #0x0001\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.eq .L_arm64_cw_free_inner\n");
    out.push_str("    movz w2, #0x0002\n");
    out.push_str("    movk w2, #0x5A11, lsl #16\n");
    out.push_str("    cmp w20, w2\n");
    out.push_str("    b.ne .L_arm64_cw_free_outer\n");

    out.push_str(".L_arm64_cw_free_inner:\n");
    out.push_str("    ldr x0, [x19, #16]\n");
    out.push_str("    cbz x0, .L_arm64_cw_free_outer\n");
    out.push_str(&format!("    bl {}free\n", p));

    out.push_str(".L_arm64_cw_free_outer:\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    bl alya_mem_track_free\n");
    out.push_str("    sub x0, x19, #16\n");
    out.push_str(&format!("    bl {}free\n", p));

    emit_adrp_add(out, "x4", "alya_gc_collected_cycles", os);
    out.push_str(".L_arm64_cw_inc_loop:\n");
    out.push_str("    ldxr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    stxr w6, x5, [x4]\n");
    out.push_str("    cbnz w6, .L_arm64_cw_inc_loop\n");

    out.push_str(".L_arm64_cw_done:\n");
    out.push_str("    ldr x25, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collect() / fn__gc_collect() -> int - ARM64
    // Executes full Bacon-Rajan Cycle Collection cycle.
    // Returns number of reclaimed cyclic objects.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_collect\n");
    out.push_str("fn_gc_collect:\n");
    out.push_str(".global fn__gc_collect\n");
    out.push_str("fn__gc_collect:\n");
    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");
    out.push_str("    str x25, [sp, #64]\n");

    // Reentrancy guard
    emit_adrp_add(out, "x1", "alya_gc_in_progress", os);
    out.push_str("    ldr x2, [x1]\n");
    out.push_str("    cbnz x2, .L_arm64_gc_reentrant\n");
    out.push_str("    mov x2, #1\n");
    out.push_str("    str x2, [x1]\n");

    emit_adrp_add(out, "x3", "alya_gc_roots_count", os);
    out.push_str("    ldr x19, [x3]\n");
    out.push_str("    cbz x19, .L_arm64_gc_early_done\n");

    emit_adrp_add(out, "x4", "alya_gc_collected_cycles", os);
    out.push_str("    ldr x20, [x4]\n");

    // Phase 1: MarkRoots
    out.push_str("    mov x21, #0\n");
    out.push_str(".L_arm64_gc_p1_loop:\n");
    out.push_str("    cmp x21, x19\n");
    out.push_str("    b.ge .L_arm64_gc_p1_done\n");
    emit_adrp_add(out, "x0", "alya_gc_roots", os);
    out.push_str("    ldr x22, [x0, x21, lsl #3]\n");
    out.push_str("    cbz x22, .L_arm64_gc_p1_next\n");
    out.push_str("    ldur w1, [x22, #-12]\n");
    out.push_str("    cmp w1, #3\n");
    out.push_str("    b.eq .L_arm64_gc_p1_call_mg\n");
    out.push_str("    cmp w1, #1\n");
    out.push_str("    b.eq .L_arm64_gc_p1_next\n");
    out.push_str("    stur wzr, [x22, #-12]\n"); // BLACK
    out.push_str("    b .L_arm64_gc_p1_next\n");
    out.push_str(".L_arm64_gc_p1_call_mg:\n");
    out.push_str("    mov x0, x22\n");
    out.push_str("    bl alya_gc_mark_gray\n");
    out.push_str(".L_arm64_gc_p1_next:\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    b .L_arm64_gc_p1_loop\n");
    out.push_str(".L_arm64_gc_p1_done:\n");

    // Phase 2: ScanRoots
    out.push_str("    mov x21, #0\n");
    out.push_str(".L_arm64_gc_p2_loop:\n");
    out.push_str("    cmp x21, x19\n");
    out.push_str("    b.ge .L_arm64_gc_p2_done\n");
    emit_adrp_add(out, "x0", "alya_gc_roots", os);
    out.push_str("    ldr x22, [x0, x21, lsl #3]\n");
    out.push_str("    cbz x22, .L_arm64_gc_p2_next\n");
    out.push_str("    mov x0, x22\n");
    out.push_str("    bl alya_gc_scan\n");
    out.push_str(".L_arm64_gc_p2_next:\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    b .L_arm64_gc_p2_loop\n");
    out.push_str(".L_arm64_gc_p2_done:\n");

    // Phase 3: CollectRoots
    out.push_str("    mov x21, #0\n");
    out.push_str(".L_arm64_gc_p3_loop:\n");
    out.push_str("    cmp x21, x19\n");
    out.push_str("    b.ge .L_arm64_gc_p3_done\n");
    emit_adrp_add(out, "x0", "alya_gc_roots", os);
    out.push_str("    ldr x22, [x0, x21, lsl #3]\n");
    out.push_str("    str xzr, [x0, x21, lsl #3]\n");
    out.push_str("    cbz x22, .L_arm64_gc_p3_next\n");
    out.push_str("    ldur w1, [x22, #-12]\n");
    out.push_str("    cmp w1, #2\n");
    out.push_str("    b.ne .L_arm64_gc_p3_next\n");
    out.push_str("    mov x0, x22\n");
    out.push_str("    bl alya_gc_collect_white\n");
    out.push_str(".L_arm64_gc_p3_next:\n");
    out.push_str("    add x21, x21, #1\n");
    out.push_str("    b .L_arm64_gc_p3_loop\n");
    out.push_str(".L_arm64_gc_p3_done:\n");

    emit_adrp_add(out, "x1", "alya_gc_roots_count", os);
    out.push_str("    str xzr, [x1]\n");
    emit_adrp_add(out, "x2", "alya_gc_in_progress", os);
    out.push_str("    str xzr, [x2]\n");
    emit_adrp_add(out, "x3", "alya_gc_collected_cycles", os);
    out.push_str("    ldr x0, [x3]\n");
    out.push_str("    sub x0, x0, x20\n");
    out.push_str("    b .L_arm64_gc_exit\n");

    out.push_str(".L_arm64_gc_early_done:\n");
    emit_adrp_add(out, "x2", "alya_gc_in_progress", os);
    out.push_str("    str xzr, [x2]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    b .L_arm64_gc_exit\n");

    out.push_str(".L_arm64_gc_reentrant:\n");
    out.push_str("    mov x0, #0\n");

    out.push_str(".L_arm64_gc_exit:\n");
    out.push_str("    ldr x25, [sp, #64]\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collected_count() / fn__gc_collected_count() -> int - ARM64
    // Returns cumulative count of swept cyclic objects.
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_collected_count\n");
    out.push_str("fn_gc_collected_count:\n");
    out.push_str(".global fn__gc_collected_count\n");
    out.push_str("fn__gc_collected_count:\n");
    emit_adrp_add(out, "x1", "alya_gc_collected_cycles", os);
    out.push_str("    ldr x0, [x1]\n");
    out.push_str("    ret\n\n");
}

fn emit_child_mark_gray(out: &mut String) {
    let lbl_next = format!(".L_arm64_mg_cn_{:x}", out.len());
    let lbl_cont = format!(".L_arm64_mg_cc_{:x}", out.len());
    let lbl_loop = format!(".L_arm64_mg_cl_{:x}", out.len());
    out.push_str("    tst x25, #7\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str("    cmp x25, #65536\n");
    out.push_str(&format!("    b.lo {}\n", lbl_next));
    out.push_str("    lsr x9, x25, #47\n");
    out.push_str(&format!("    cbnz x9, {}\n", lbl_next));
    out.push_str("    ldur w9, [x25, #-16]\n");
    out.push_str("    movz w10, #0x0001\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0002\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0003\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    sub x10, x25, #8\n");
    out.push_str(&format!("{}:\n", lbl_loop));
    out.push_str("    ldxr x11, [x10]\n");
    out.push_str("    sub x11, x11, #1\n");
    out.push_str("    stxr w12, x11, [x10]\n");
    out.push_str(&format!("    cbnz w12, {}\n", lbl_loop));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl alya_gc_mark_gray\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan(out: &mut String) {
    let lbl_next = format!(".L_arm64_s_cn_{:x}", out.len());
    let lbl_cont = format!(".L_arm64_s_cc_{:x}", out.len());
    out.push_str("    tst x25, #7\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str("    cmp x25, #65536\n");
    out.push_str(&format!("    b.lo {}\n", lbl_next));
    out.push_str("    lsr x9, x25, #47\n");
    out.push_str(&format!("    cbnz x9, {}\n", lbl_next));
    out.push_str("    ldur w9, [x25, #-16]\n");
    out.push_str("    movz w10, #0x0001\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0002\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0003\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl alya_gc_scan\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan_black(out: &mut String) {
    let lbl_next = format!(".L_arm64_sb_cn_{:x}", out.len());
    let lbl_cont = format!(".L_arm64_sb_cc_{:x}", out.len());
    let lbl_loop = format!(".L_arm64_sb_cl_{:x}", out.len());
    out.push_str("    tst x25, #7\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str("    cmp x25, #65536\n");
    out.push_str(&format!("    b.lo {}\n", lbl_next));
    out.push_str("    lsr x9, x25, #47\n");
    out.push_str(&format!("    cbnz x9, {}\n", lbl_next));
    out.push_str("    ldur w9, [x25, #-16]\n");
    out.push_str("    movz w10, #0x0001\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0002\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0003\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    sub x10, x25, #8\n");
    out.push_str(&format!("{}:\n", lbl_loop));
    out.push_str("    ldxr x11, [x10]\n");
    out.push_str("    add x11, x11, #1\n");
    out.push_str("    stxr w12, x11, [x10]\n");
    out.push_str(&format!("    cbnz w12, {}\n", lbl_loop));
    out.push_str("    ldur w10, [x25, #-12]\n");
    out.push_str(&format!("    cbz w10, {}\n", lbl_next));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl alya_gc_scan_black\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_collect_white(out: &mut String) {
    let lbl_next = format!(".L_arm64_cw_cn_{:x}", out.len());
    let lbl_cont = format!(".L_arm64_cw_cc_{:x}", out.len());
    let lbl_str = format!(".L_arm64_cw_cs_{:x}", out.len());
    let lbl_ext = format!(".L_arm64_cw_ce_{:x}", out.len());
    out.push_str("    tst x25, #7\n");
    out.push_str(&format!("    b.ne {}\n", lbl_next));
    out.push_str("    cmp x25, #65536\n");
    out.push_str(&format!("    b.lo {}\n", lbl_next));
    out.push_str("    lsr x9, x25, #47\n");
    out.push_str(&format!("    cbnz x9, {}\n", lbl_next));
    out.push_str("    ldur w9, [x25, #-16]\n");
    out.push_str("    movz w10, #0x0001\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0002\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0003\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_cont));
    out.push_str("    movz w10, #0x0004\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str(&format!("    b.eq {}\n", lbl_str));
    out.push_str(&format!("    b {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    ldur w10, [x25, #-12]\n");
    out.push_str("    cmp w10, #2\n");
    out.push_str(&format!("    b.ne {}\n", lbl_ext));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl alya_gc_collect_white\n");
    out.push_str(&format!("    b {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_ext));
    out.push_str("    ldur x10, [x25, #-8]\n");
    out.push_str("    cmp x10, #0\n");
    out.push_str(&format!("    b.le {}\n", lbl_next));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl fn_rc_release\n");
    out.push_str(&format!("    b {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_str));
    out.push_str("    mov x0, x25\n");
    out.push_str("    bl fn_rc_release\n");
    out.push_str(&format!("{}:\n", lbl_next));
}
