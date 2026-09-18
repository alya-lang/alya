use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // alya_mem_track_alloc(ptr, size, kind, desc)
    // =========================================================================
    out.push_str(".global alya_mem_track_alloc\n");
    out.push_str("alya_mem_track_alloc:\n");
    out.push_str("    cmpl $0, alya_mem_trace_enabled\n");
    out.push_str("    jz .L_x86_tr_alloc_ret\n");
    out.push_str("    mov 4(%esp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_tr_alloc_ret\n");

    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    and $-16, %esp\n");
    out.push_str("    sub $16, %esp\n");

    out.push_str("    mov 8(%ebp), %esi\n");  // ptr
    out.push_str("    mov 12(%ebp), %edi\n"); // size
    out.push_str("    mov 16(%ebp), %ebx\n"); // kind

    out.push_str("    lock incl alya_mem_total_allocs\n");
    out.push_str("    lock incl alya_mem_active_allocs\n");
    out.push_str("    lock addl %edi, alya_mem_total_bytes\n");
    out.push_str("    lock addl %edi, alya_mem_active_bytes\n");

    out.push_str("    mov alya_mem_active_bytes, %eax\n");
    out.push_str("    cmpl alya_mem_peak_bytes, %eax\n");
    out.push_str("    jbe .L_x86_peak_ok\n");
    out.push_str("    mov %eax, alya_mem_peak_bytes\n");
    out.push_str(".L_x86_peak_ok:\n");

    out.push_str("    cmpl $1, %ebx\n");
    out.push_str("    jne .L_x86_k2\n");
    out.push_str("    lock incl alya_mem_live_arrays\n");
    out.push_str("    jmp .L_x86_k_done\n");
    out.push_str(".L_x86_k2:\n");
    out.push_str("    cmpl $2, %ebx\n");
    out.push_str("    jne .L_x86_k3\n");
    out.push_str("    lock incl alya_mem_live_maps\n");
    out.push_str("    jmp .L_x86_k_done\n");
    out.push_str(".L_x86_k3:\n");
    out.push_str("    cmpl $3, %ebx\n");
    out.push_str("    jne .L_x86_k4\n");
    out.push_str("    lock incl alya_mem_live_structs\n");
    out.push_str("    jmp .L_x86_k_done\n");
    out.push_str(".L_x86_k4:\n");
    out.push_str("    cmpl $4, %ebx\n");
    out.push_str("    jne .L_x86_k5\n");
    out.push_str("    lock incl alya_mem_live_strings\n");
    out.push_str("    jmp .L_x86_k_done\n");
    out.push_str(".L_x86_k5:\n");
    out.push_str("    lock incl alya_mem_live_raw\n");
    out.push_str(".L_x86_k_done:\n");

    // Allocate 24-byte record node for 32-bit pointers
    out.push_str("    push $24\n");
    out.push_str(&format!("    call {}malloc\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_tr_alloc_fail\n");

    out.push_str("    mov %esi, 0(%eax)\n");
    out.push_str("    mov %edi, 4(%eax)\n");
    out.push_str("    mov %ebx, 8(%eax)\n");
    out.push_str("    mov 20(%ebp), %edx\n");
    out.push_str("    mov %edx, 12(%eax)\n");

    out.push_str("    mov alya_mem_records_head, %edx\n");
    out.push_str("    mov %edx, 16(%eax)\n");
    out.push_str("    movl $0, 20(%eax)\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_set_head\n");
    out.push_str("    mov %eax, 20(%edx)\n");
    out.push_str(".L_x86_set_head:\n");
    out.push_str("    mov %eax, alya_mem_records_head\n");

    out.push_str(".L_x86_tr_alloc_fail:\n");
    out.push_str("    lea -12(%ebp), %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_tr_alloc_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_track_free(ptr)
    // =========================================================================
    out.push_str(".global alya_mem_track_free\n");
    out.push_str("alya_mem_track_free:\n");
    out.push_str("    cmpl $0, alya_mem_trace_enabled\n");
    out.push_str("    jz .L_x86_tr_free_ret\n");
    out.push_str("    mov 4(%esp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_tr_free_ret\n");

    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    and $-16, %esp\n");
    out.push_str("    sub $16, %esp\n");

    out.push_str("    mov 8(%ebp), %esi\n"); // target ptr
    out.push_str("    mov alya_mem_records_head, %ebx\n"); // cur = head

    out.push_str(".L_x86_free_search:\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_free_not_found\n");
    out.push_str("    cmpl %esi, 0(%ebx)\n");
    out.push_str("    je .L_x86_free_found\n");
    out.push_str("    mov 16(%ebx), %ebx\n");
    out.push_str("    jmp .L_x86_free_search\n");

    out.push_str(".L_x86_free_found:\n");
    out.push_str("    mov 16(%ebx), %ecx\n"); // next
    out.push_str("    mov 20(%ebx), %edx\n"); // prev

    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_free_head\n");
    out.push_str("    mov %ecx, 16(%edx)\n");
    out.push_str("    jmp .L_x86_free_chk_next\n");
    out.push_str(".L_x86_free_head:\n");
    out.push_str("    mov %ecx, alya_mem_records_head\n");

    out.push_str(".L_x86_free_chk_next:\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_free_unlinked\n");
    out.push_str("    mov %edx, 20(%ecx)\n");

    out.push_str(".L_x86_free_unlinked:\n");
    out.push_str("    lock incl alya_mem_total_frees\n");
    out.push_str("    lock decl alya_mem_active_allocs\n");

    out.push_str("    mov 4(%ebx), %eax\n"); // size
    out.push_str("    lock subl %eax, alya_mem_active_bytes\n");

    out.push_str("    mov 8(%ebx), %eax\n"); // kind
    out.push_str("    cmpl $1, %eax\n");
    out.push_str("    jne .L_x86_dec_k2\n");
    out.push_str("    lock decl alya_mem_live_arrays\n");
    out.push_str("    jmp .L_x86_dec_done\n");
    out.push_str(".L_x86_dec_k2:\n");
    out.push_str("    cmpl $2, %eax\n");
    out.push_str("    jne .L_x86_dec_k3\n");
    out.push_str("    lock decl alya_mem_live_maps\n");
    out.push_str("    jmp .L_x86_dec_done\n");
    out.push_str(".L_x86_dec_k3:\n");
    out.push_str("    cmpl $3, %eax\n");
    out.push_str("    jne .L_x86_dec_k4\n");
    out.push_str("    lock decl alya_mem_live_structs\n");
    out.push_str("    jmp .L_x86_dec_done\n");
    out.push_str(".L_x86_dec_k4:\n");
    out.push_str("    cmpl $4, %eax\n");
    out.push_str("    jne .L_x86_dec_k5\n");
    out.push_str("    lock decl alya_mem_live_strings\n");
    out.push_str("    jmp .L_x86_dec_done\n");
    out.push_str(".L_x86_dec_k5:\n");
    out.push_str("    lock decl alya_mem_live_raw\n");
    out.push_str(".L_x86_dec_done:\n");

    out.push_str("    push %ebx\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");

    out.push_str(".L_x86_free_not_found:\n");
    out.push_str("    lea -12(%ebp), %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_tr_free_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_trace_report()
    // =========================================================================
    out.push_str(".global alya_mem_trace_report\n");
    out.push_str("alya_mem_trace_report:\n");
    out.push_str("    cmpl $0, alya_mem_trace_enabled\n");
    out.push_str("    jz .L_x86_rep_ret\n");
    out.push_str("    cmpl $0, alya_mem_report_done\n");
    out.push_str("    jnz .L_x86_rep_ret\n");
    out.push_str("    movl $1, alya_mem_report_done\n");

    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    and $-16, %esp\n");
    out.push_str("    sub $32, %esp\n");

    // 1. Header
    out.push_str("    push $alya_mem_fmt_header\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $4, %esp\n");

    // 2. Totals
    out.push_str("    push alya_mem_active_allocs\n");
    out.push_str("    push alya_mem_total_frees\n");
    out.push_str("    push alya_mem_total_allocs\n");
    out.push_str("    push $alya_mem_fmt_totals\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");

    // 3. Bytes
    out.push_str("    push alya_mem_active_bytes\n");
    out.push_str("    push alya_mem_peak_bytes\n");
    out.push_str("    push alya_mem_total_bytes\n");
    out.push_str("    push $alya_mem_fmt_bytes\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");

    // 4. Objects
    out.push_str("    push alya_mem_live_raw\n");
    out.push_str("    push alya_mem_live_strings\n");
    out.push_str("    push alya_mem_live_maps\n");
    out.push_str("    push alya_mem_live_arrays\n");
    out.push_str("    push alya_mem_live_structs\n");
    out.push_str("    push $alya_mem_fmt_objects\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $24, %esp\n");

    // 5. Clean vs Leaks
    out.push_str("    mov alya_mem_active_allocs, %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_rep_has_leaks\n");

    out.push_str("    push $alya_mem_fmt_clean\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_rep_flush\n");

    out.push_str(".L_x86_rep_has_leaks:\n");
    out.push_str("    push alya_mem_active_bytes\n");
    out.push_str("    push alya_mem_active_allocs\n");
    out.push_str("    push $alya_mem_fmt_warn\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $12, %esp\n");

    out.push_str("    mov alya_mem_records_head, %ebx\n");
    out.push_str("    mov $1, %esi\n"); // index = 1

    out.push_str(".L_x86_rep_loop:\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_rep_footer\n");
    out.push_str("    cmpl $50, %esi\n");
    out.push_str("    jg .L_x86_rep_footer\n");

    out.push_str("    mov 0(%ebx), %edi\n"); // ptr
    out.push_str("    mov 4(%ebx), %edx\n"); // size
    out.push_str("    mov 8(%ebx), %eax\n"); // kind
    out.push_str("    mov 12(%ebx), %ecx\n"); // desc

    out.push_str("    cmpl $3, %eax\n");
    out.push_str("    jne .L_x86_rep_chk_arr\n");

    // Struct
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jnz .L_x86_rep_desc_ok\n");
    out.push_str("    mov $alya_str_anon_struct, %ecx\n");
    out.push_str("    jmp .L_x86_rep_desc_done\n");
    out.push_str(".L_x86_rep_desc_ok:\n");
    out.push_str("    mov (%ecx), %ecx\n"); // struct name string
    out.push_str(".L_x86_rep_desc_done:\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $alya_mem_fmt_item_struct\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $20, %esp\n");
    out.push_str("    jmp .L_x86_rep_loop_next\n");

    out.push_str(".L_x86_rep_chk_arr:\n");
    out.push_str("    cmpl $1, %eax\n");
    out.push_str("    jne .L_x86_rep_chk_map\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $alya_mem_fmt_item_array\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");
    out.push_str("    jmp .L_x86_rep_loop_next\n");

    out.push_str(".L_x86_rep_chk_map:\n");
    out.push_str("    cmpl $2, %eax\n");
    out.push_str("    jne .L_x86_rep_chk_str\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $alya_mem_fmt_item_map\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");
    out.push_str("    jmp .L_x86_rep_loop_next\n");

    out.push_str(".L_x86_rep_chk_str:\n");
    out.push_str("    cmpl $4, %eax\n");
    out.push_str("    jne .L_x86_rep_raw\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $alya_mem_fmt_item_str\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");
    out.push_str("    jmp .L_x86_rep_loop_next\n");

    out.push_str(".L_x86_rep_raw:\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $alya_mem_fmt_item_raw\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $16, %esp\n");

    out.push_str(".L_x86_rep_loop_next:\n");
    out.push_str("    incl %esi\n");
    out.push_str("    mov 16(%ebx), %ebx\n");
    out.push_str("    jmp .L_x86_rep_loop\n");

    out.push_str(".L_x86_rep_footer:\n");
    out.push_str("    push $alya_mem_fmt_footer\n");
    out.push_str(&format!("    call {}printf\n", p));
    out.push_str("    add $4, %esp\n");

    out.push_str(".L_x86_rep_flush:\n");
    out.push_str("    push $0\n");
    out.push_str(&format!("    call {}fflush\n", p));
    out.push_str("    add $4, %esp\n");

    out.push_str("    lea -12(%ebp), %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_rep_ret:\n");
    out.push_str("    ret\n\n");
}
