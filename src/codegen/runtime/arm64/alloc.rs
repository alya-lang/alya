use crate::codegen::arch::arm64::emit_adrp_add;
use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // alya_mem_track_alloc(x0: ptr, x1: size, x2: kind, x3: desc)
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global alya_mem_track_alloc\n");
    out.push_str("alya_mem_track_alloc:\n");
    emit_adrp_add(out, "x4", "alya_mem_trace_enabled", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    cbz x5, .L_arm64_tr_alloc_ret\n");
    out.push_str("    cbz x0, .L_arm64_tr_alloc_ret\n");

    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");

    out.push_str("    mov x19, x0\n"); // ptr
    out.push_str("    mov x20, x1\n"); // size
    out.push_str("    mov x21, x2\n"); // kind
    out.push_str("    mov x22, x3\n"); // desc

    // Total allocs & Active allocs
    emit_adrp_add(out, "x4", "alya_mem_total_allocs", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");

    emit_adrp_add(out, "x4", "alya_mem_active_allocs", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");

    // Total bytes & Active bytes
    emit_adrp_add(out, "x4", "alya_mem_total_bytes", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, x20\n");
    out.push_str("    str x5, [x4]\n");

    emit_adrp_add(out, "x4", "alya_mem_active_bytes", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, x20\n");
    out.push_str("    str x5, [x4]\n");

    // Peak bytes
    emit_adrp_add(out, "x6", "alya_mem_peak_bytes", os);
    out.push_str("    ldr x7, [x6]\n");
    out.push_str("    cmp x5, x7\n");
    out.push_str("    b.ls .L_arm64_peak_ok\n");
    out.push_str("    str x5, [x6]\n");
    out.push_str(".L_arm64_peak_ok:\n");

    // Kinds: 1: Array, 2: Map, 3: Struct, 4: String, 5: Raw
    out.push_str("    cmp x21, #1\n");
    out.push_str("    b.ne .L_arm64_k2\n");
    emit_adrp_add(out, "x4", "alya_mem_live_arrays", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_k_done\n");
    out.push_str(".L_arm64_k2:\n");
    out.push_str("    cmp x21, #2\n");
    out.push_str("    b.ne .L_arm64_k3\n");
    emit_adrp_add(out, "x4", "alya_mem_live_maps", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_k_done\n");
    out.push_str(".L_arm64_k3:\n");
    out.push_str("    cmp x21, #3\n");
    out.push_str("    b.ne .L_arm64_k4\n");
    emit_adrp_add(out, "x4", "alya_mem_live_structs", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_k_done\n");
    out.push_str(".L_arm64_k4:\n");
    out.push_str("    cmp x21, #4\n");
    out.push_str("    b.ne .L_arm64_k5\n");
    emit_adrp_add(out, "x4", "alya_mem_live_strings", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_k_done\n");
    out.push_str(".L_arm64_k5:\n");
    emit_adrp_add(out, "x4", "alya_mem_live_raw", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str(".L_arm64_k_done:\n");

    // Allocate 48-byte tracking record node
    out.push_str("    mov x0, #48\n");
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    cbz x0, .L_arm64_tr_alloc_exit\n");

    out.push_str("    str x19, [x0, #0]\n");
    out.push_str("    str x20, [x0, #8]\n");
    out.push_str("    str x21, [x0, #16]\n");
    out.push_str("    str x22, [x0, #24]\n");

    emit_adrp_add(out, "x4", "alya_mem_records_head", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    str x5, [x0, #32]\n"); // node->next = head
    out.push_str("    str xzr, [x0, #40]\n"); // node->prev = NULL
    out.push_str("    cbz x5, .L_arm64_set_head\n");
    out.push_str("    str x0, [x5, #40]\n"); // head->prev = node
    out.push_str(".L_arm64_set_head:\n");
    out.push_str("    str x0, [x4]\n");

    out.push_str(".L_arm64_tr_alloc_exit:\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_tr_alloc_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_track_free(x0: ptr)
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global alya_mem_track_free\n");
    out.push_str("alya_mem_track_free:\n");
    emit_adrp_add(out, "x4", "alya_mem_trace_enabled", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    cbz x5, .L_arm64_tr_free_ret\n");
    out.push_str("    cbz x0, .L_arm64_tr_free_ret\n");

    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");

    out.push_str("    mov x19, x0\n"); // target ptr
    emit_adrp_add(out, "x4", "alya_mem_records_head", os);
    out.push_str("    ldr x20, [x4]\n"); // cur = head

    out.push_str(".L_arm64_free_search:\n");
    out.push_str("    cbz x20, .L_arm64_free_not_found\n");
    out.push_str("    ldr x1, [x20, #0]\n");
    out.push_str("    cmp x1, x19\n");
    out.push_str("    b.eq .L_arm64_free_found\n");
    out.push_str("    ldr x20, [x20, #32]\n");
    out.push_str("    b .L_arm64_free_search\n");

    out.push_str(".L_arm64_free_found:\n");
    out.push_str("    ldr x21, [x20, #32]\n"); // next
    out.push_str("    ldr x22, [x20, #40]\n"); // prev

    out.push_str("    cbnz x22, .L_arm64_free_has_prev\n");
    emit_adrp_add(out, "x4", "alya_mem_records_head", os);
    out.push_str("    str x21, [x4]\n");
    out.push_str("    b .L_arm64_free_chk_next\n");
    out.push_str(".L_arm64_free_has_prev:\n");
    out.push_str("    str x21, [x22, #32]\n");

    out.push_str(".L_arm64_free_chk_next:\n");
    out.push_str("    cbz x21, .L_arm64_free_unlinked\n");
    out.push_str("    str x22, [x21, #40]\n");

    out.push_str(".L_arm64_free_unlinked:\n");
    emit_adrp_add(out, "x4", "alya_mem_total_frees", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    add x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");

    emit_adrp_add(out, "x4", "alya_mem_active_allocs", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");

    out.push_str("    ldr x6, [x20, #8]\n"); // size
    emit_adrp_add(out, "x4", "alya_mem_active_bytes", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, x6\n");
    out.push_str("    str x5, [x4]\n");

    out.push_str("    ldr x7, [x20, #16]\n"); // kind
    out.push_str("    cmp x7, #1\n");
    out.push_str("    b.ne .L_arm64_f_k2\n");
    emit_adrp_add(out, "x4", "alya_mem_live_arrays", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_f_k_done\n");
    out.push_str(".L_arm64_f_k2:\n");
    out.push_str("    cmp x7, #2\n");
    out.push_str("    b.ne .L_arm64_f_k3\n");
    emit_adrp_add(out, "x4", "alya_mem_live_maps", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_f_k_done\n");
    out.push_str(".L_arm64_f_k3:\n");
    out.push_str("    cmp x7, #3\n");
    out.push_str("    b.ne .L_arm64_f_k4\n");
    emit_adrp_add(out, "x4", "alya_mem_live_structs", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_f_k_done\n");
    out.push_str(".L_arm64_f_k4:\n");
    out.push_str("    cmp x7, #4\n");
    out.push_str("    b.ne .L_arm64_f_k5\n");
    emit_adrp_add(out, "x4", "alya_mem_live_strings", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str("    b .L_arm64_f_k_done\n");
    out.push_str(".L_arm64_f_k5:\n");
    emit_adrp_add(out, "x4", "alya_mem_live_raw", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    sub x5, x5, #1\n");
    out.push_str("    str x5, [x4]\n");
    out.push_str(".L_arm64_f_k_done:\n");

    out.push_str("    mov x0, x20\n");
    out.push_str(&format!("    bl {}free\n", p));

    out.push_str(".L_arm64_free_not_found:\n");
    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_tr_free_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_trace_report()
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global alya_mem_trace_report\n");
    out.push_str("alya_mem_trace_report:\n");
    emit_adrp_add(out, "x4", "alya_mem_trace_enabled", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    cbz x5, .L_arm64_rep_ret\n");
    emit_adrp_add(out, "x4", "alya_mem_report_done", os);
    out.push_str("    ldr x5, [x4]\n");
    out.push_str("    cbnz x5, .L_arm64_rep_ret\n");
    out.push_str("    mov x5, #1\n");
    out.push_str("    str x5, [x4]\n");

    out.push_str("    stp x29, x30, [sp, #-80]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    stp x23, x24, [sp, #48]\n");

    // 1. Header
    emit_adrp_add(out, "x0", "alya_mem_fmt_header", os);
    out.push_str(&format!("    bl {}printf\n", p));

    // 2. Totals
    emit_adrp_add(out, "x0", "alya_mem_fmt_totals", os);
    emit_adrp_add(out, "x4", "alya_mem_total_allocs", os);
    out.push_str("    ldr x1, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_total_frees", os);
    out.push_str("    ldr x2, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_active_allocs", os);
    out.push_str("    ldr x3, [x4]\n");
    out.push_str(&format!("    bl {}printf\n", p));

    // 3. Bytes
    emit_adrp_add(out, "x0", "alya_mem_fmt_bytes", os);
    emit_adrp_add(out, "x4", "alya_mem_total_bytes", os);
    out.push_str("    ldr x1, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_peak_bytes", os);
    out.push_str("    ldr x2, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_active_bytes", os);
    out.push_str("    ldr x3, [x4]\n");
    out.push_str(&format!("    bl {}printf\n", p));

    // 4. Live Objects
    emit_adrp_add(out, "x0", "alya_mem_fmt_objects", os);
    emit_adrp_add(out, "x4", "alya_mem_live_structs", os);
    out.push_str("    ldr x1, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_live_arrays", os);
    out.push_str("    ldr x2, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_live_maps", os);
    out.push_str("    ldr x3, [x4]\n");
    emit_adrp_add(out, "x4", "alya_mem_live_strings", os);
    out.push_str("    ldr x4, [x4]\n");
    emit_adrp_add(out, "x5", "alya_mem_live_raw", os);
    out.push_str("    ldr x5, [x5]\n");
    out.push_str(&format!("    bl {}printf\n", p));

    // 5. Clean vs Leaks
    emit_adrp_add(out, "x4", "alya_mem_active_allocs", os);
    out.push_str("    ldr x1, [x4]\n");
    out.push_str("    cbnz x1, .L_arm64_rep_has_leaks\n");

    emit_adrp_add(out, "x0", "alya_mem_fmt_clean", os);
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_rep_flush\n");

    out.push_str(".L_arm64_rep_has_leaks:\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_warn", os);
    emit_adrp_add(out, "x4", "alya_mem_active_bytes", os);
    out.push_str("    ldr x2, [x4]\n");
    out.push_str(&format!("    bl {}printf\n", p));

    emit_adrp_add(out, "x4", "alya_mem_records_head", os);
    out.push_str("    ldr x19, [x4]\n"); // cur = head
    out.push_str("    mov x20, #1\n");   // counter = 1

    out.push_str(".L_arm64_rep_loop:\n");
    out.push_str("    cbz x19, .L_arm64_rep_footer\n");
    out.push_str("    cmp x20, #50\n");
    out.push_str("    b.gt .L_arm64_rep_footer\n");

    out.push_str("    ldr x21, [x19, #0]\n");  // ptr
    out.push_str("    ldr x22, [x19, #8]\n");  // size
    out.push_str("    ldr x23, [x19, #16]\n"); // kind
    out.push_str("    ldr x24, [x19, #24]\n"); // desc

    out.push_str("    cmp x23, #3\n");
    out.push_str("    b.ne .L_arm64_rep_chk_arr\n");

    out.push_str("    cbnz x24, .L_arm64_rep_desc_ok\n");
    emit_adrp_add(out, "x24", "alya_str_anon_struct", os);
    out.push_str("    b .L_arm64_rep_desc_done\n");
    out.push_str(".L_arm64_rep_desc_ok:\n");
    out.push_str("    ldr x24, [x24]\n");
    out.push_str(".L_arm64_rep_desc_done:\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_item_struct", os);
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x21\n");
    out.push_str("    mov x3, x22\n");
    out.push_str("    mov x4, x24\n");
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_rep_loop_next\n");

    out.push_str(".L_arm64_rep_chk_arr:\n");
    out.push_str("    cmp x23, #1\n");
    out.push_str("    b.ne .L_arm64_rep_chk_map\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_item_array", os);
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x21\n");
    out.push_str("    mov x3, x22\n");
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_rep_loop_next\n");

    out.push_str(".L_arm64_rep_chk_map:\n");
    out.push_str("    cmp x23, #2\n");
    out.push_str("    b.ne .L_arm64_rep_chk_str\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_item_map", os);
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x21\n");
    out.push_str("    mov x3, x22\n");
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_rep_loop_next\n");

    out.push_str(".L_arm64_rep_chk_str:\n");
    out.push_str("    cmp x23, #4\n");
    out.push_str("    b.ne .L_arm64_rep_raw\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_item_str", os);
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x21\n");
    out.push_str("    mov x3, x22\n");
    out.push_str(&format!("    bl {}printf\n", p));
    out.push_str("    b .L_arm64_rep_loop_next\n");

    out.push_str(".L_arm64_rep_raw:\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_item_raw", os);
    out.push_str("    mov x1, x20\n");
    out.push_str("    mov x2, x21\n");
    out.push_str("    mov x3, x22\n");
    out.push_str(&format!("    bl {}printf\n", p));

    out.push_str(".L_arm64_rep_loop_next:\n");
    out.push_str("    add x20, x20, #1\n");
    out.push_str("    ldr x19, [x19, #32]\n");
    out.push_str("    b .L_arm64_rep_loop\n");

    out.push_str(".L_arm64_rep_footer:\n");
    emit_adrp_add(out, "x0", "alya_mem_fmt_footer", os);
    out.push_str(&format!("    bl {}printf\n", p));

    out.push_str(".L_arm64_rep_flush:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    bl {}fflush\n", p));

    out.push_str("    ldp x23, x24, [sp, #48]\n");
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #80\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_rep_ret:\n");
    out.push_str("    ret\n\n");
}
