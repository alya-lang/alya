use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // alya_mem_track_alloc(ptr, size, kind, desc)
    // =========================================================================
    out.push_str(".global alya_mem_track_alloc\n");
    out.push_str("alya_mem_track_alloc:\n");
    out.push_str("    cmpq $0, alya_mem_trace_enabled(%rip)\n");
    out.push_str("    jz .L_mem_tr_alloc_ret\n");
    if is_win {
        out.push_str("    test %rcx, %rcx\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
    }
    out.push_str("    jz .L_mem_tr_alloc_ret\n");

    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    and $-16, %rsp\n");
    out.push_str("    sub $48, %rsp\n");

    // Save inputs: r12=ptr, r13=size, r14=kind, r15=desc
    if is_win {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    mov %r8, %r14\n");
        out.push_str("    mov %r9, %r15\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
        out.push_str("    mov %rdx, %r14\n");
        out.push_str("    mov %rcx, %r15\n");
    }

    // Atomic / Global updates
    out.push_str("    lock incq alya_mem_total_allocs(%rip)\n");
    out.push_str("    lock incq alya_mem_active_allocs(%rip)\n");
    out.push_str("    lock addq %r13, alya_mem_total_bytes(%rip)\n");
    out.push_str("    lock addq %r13, alya_mem_active_bytes(%rip)\n");

    out.push_str("    movq alya_mem_active_bytes(%rip), %rax\n");
    out.push_str("    cmpq alya_mem_peak_bytes(%rip), %rax\n");
    out.push_str("    jbe .L_mem_peak_ok\n");
    out.push_str("    movq %rax, alya_mem_peak_bytes(%rip)\n");
    out.push_str(".L_mem_peak_ok:\n");

    // Kind counters (1: Array, 2: Map, 3: Struct, 4: String, 5: Raw)
    out.push_str("    cmpq $1, %r14\n");
    out.push_str("    jne .L_mem_k2\n");
    out.push_str("    lock incq alya_mem_live_arrays(%rip)\n");
    out.push_str("    jmp .L_mem_k_done\n");
    out.push_str(".L_mem_k2:\n");
    out.push_str("    cmpq $2, %r14\n");
    out.push_str("    jne .L_mem_k3\n");
    out.push_str("    lock incq alya_mem_live_maps(%rip)\n");
    out.push_str("    jmp .L_mem_k_done\n");
    out.push_str(".L_mem_k3:\n");
    out.push_str("    cmpq $3, %r14\n");
    out.push_str("    jne .L_mem_k4\n");
    out.push_str("    lock incq alya_mem_live_structs(%rip)\n");
    out.push_str("    jmp .L_mem_k_done\n");
    out.push_str(".L_mem_k4:\n");
    out.push_str("    cmpq $4, %r14\n");
    out.push_str("    jne .L_mem_k5\n");
    out.push_str("    lock incq alya_mem_live_strings(%rip)\n");
    out.push_str("    jmp .L_mem_k_done\n");
    out.push_str(".L_mem_k5:\n");
    out.push_str("    lock incq alya_mem_live_raw(%rip)\n");
    out.push_str(".L_mem_k_done:\n");

    // Allocate tracking node: 48 bytes
    if is_win {
        out.push_str("    mov $48, %rcx\n");
        out.push_str("    call malloc\n");
    } else {
        out.push_str("    mov $48, %rdi\n");
        out.push_str(&format!("    call {}malloc\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_mem_alloc_node_fail\n");

    out.push_str("    movq %r12, 0(%rax)\n");  // node->ptr = ptr
    out.push_str("    movq %r13, 8(%rax)\n");  // node->size = size
    out.push_str("    movq %r14, 16(%rax)\n"); // node->kind = kind
    out.push_str("    movq %r15, 24(%rax)\n"); // node->desc = desc
    out.push_str("    movq alya_mem_records_head(%rip), %rdx\n");
    out.push_str("    movq %rdx, 32(%rax)\n"); // node->next = head
    out.push_str("    movq $0, 40(%rax)\n");   // node->prev = 0
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_mem_set_head\n");
    out.push_str("    movq %rax, 40(%rdx)\n"); // head->prev = node
    out.push_str(".L_mem_set_head:\n");
    out.push_str("    movq %rax, alya_mem_records_head(%rip)\n");

    out.push_str(".L_mem_alloc_node_fail:\n");
    out.push_str("    lea -56(%rbp), %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_mem_tr_alloc_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_track_free(ptr)
    // =========================================================================
    out.push_str(".global alya_mem_track_free\n");
    out.push_str("alya_mem_track_free:\n");
    out.push_str("    cmpq $0, alya_mem_trace_enabled(%rip)\n");
    out.push_str("    jz .L_mem_tr_free_ret\n");
    if is_win {
        out.push_str("    test %rcx, %rcx\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
    }
    out.push_str("    jz .L_mem_tr_free_ret\n");

    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    and $-16, %rsp\n");
    out.push_str("    sub $48, %rsp\n");

    if is_win {
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
    }

    out.push_str("    movq alya_mem_records_head(%rip), %rbx\n");
    out.push_str(".L_mem_free_search:\n");
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_mem_free_not_found\n");
    out.push_str("    cmpq %r12, 0(%rbx)\n");
    out.push_str("    je .L_mem_free_found\n");
    out.push_str("    movq 32(%rbx), %rbx\n");
    out.push_str("    jmp .L_mem_free_search\n");

    out.push_str(".L_mem_free_found:\n");
    out.push_str("    movq 32(%rbx), %r13\n"); // next
    out.push_str("    movq 40(%rbx), %r14\n"); // prev

    out.push_str("    test %r14, %r14\n");
    out.push_str("    jz .L_mem_free_head\n");
    out.push_str("    movq %r13, 32(%r14)\n"); // prev->next = next
    out.push_str("    jmp .L_mem_free_chk_next\n");
    out.push_str(".L_mem_free_head:\n");
    out.push_str("    movq %r13, alya_mem_records_head(%rip)\n");

    out.push_str(".L_mem_free_chk_next:\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_mem_free_unlinked\n");
    out.push_str("    movq %r14, 40(%r13)\n"); // next->prev = prev

    out.push_str(".L_mem_free_unlinked:\n");
    out.push_str("    lock incq alya_mem_total_frees(%rip)\n");
    out.push_str("    lock decq alya_mem_active_allocs(%rip)\n");
    out.push_str("    movq 8(%rbx), %rax\n");
    out.push_str("    lock subq %rax, alya_mem_active_bytes(%rip)\n");

    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    cmpq $1, %rax\n");
    out.push_str("    jne .L_mem_dec_k2\n");
    out.push_str("    lock decq alya_mem_live_arrays(%rip)\n");
    out.push_str("    jmp .L_mem_dec_done\n");
    out.push_str(".L_mem_dec_k2:\n");
    out.push_str("    cmpq $2, %rax\n");
    out.push_str("    jne .L_mem_dec_k3\n");
    out.push_str("    lock decq alya_mem_live_maps(%rip)\n");
    out.push_str("    jmp .L_mem_dec_done\n");
    out.push_str(".L_mem_dec_k3:\n");
    out.push_str("    cmpq $3, %rax\n");
    out.push_str("    jne .L_mem_dec_k4\n");
    out.push_str("    lock decq alya_mem_live_structs(%rip)\n");
    out.push_str("    jmp .L_mem_dec_done\n");
    out.push_str(".L_mem_dec_k4:\n");
    out.push_str("    cmpq $4, %rax\n");
    out.push_str("    jne .L_mem_dec_k5\n");
    out.push_str("    lock decq alya_mem_live_strings(%rip)\n");
    out.push_str("    jmp .L_mem_dec_done\n");
    out.push_str(".L_mem_dec_k5:\n");
    out.push_str("    lock decq alya_mem_live_raw(%rip)\n");
    out.push_str(".L_mem_dec_done:\n");

    // Free node
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }

    out.push_str(".L_mem_free_not_found:\n");
    out.push_str("    lea -56(%rbp), %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_mem_tr_free_ret:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_mem_trace_report()
    // =========================================================================
    out.push_str(".global alya_mem_trace_report\n");
    out.push_str("alya_mem_trace_report:\n");
    out.push_str("    cmpq $0, alya_mem_trace_enabled(%rip)\n");
    out.push_str("    jz .L_mem_rep_ret\n");
    out.push_str("    cmpq $0, alya_mem_report_done(%rip)\n");
    out.push_str("    jnz .L_mem_rep_ret\n");
    out.push_str("    movq $1, alya_mem_report_done(%rip)\n");

    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    and $-16, %rsp\n");
    out.push_str("    sub $64, %rsp\n");

    // 1. Header
    if is_win {
        out.push_str("    lea alya_mem_fmt_header(%rip), %rcx\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_header(%rip), %rdi\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    // 2. Totals
    if is_win {
        out.push_str("    lea alya_mem_fmt_totals(%rip), %rcx\n");
        out.push_str("    movq alya_mem_total_allocs(%rip), %rdx\n");
        out.push_str("    movq alya_mem_total_frees(%rip), %r8\n");
        out.push_str("    movq alya_mem_active_allocs(%rip), %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_totals(%rip), %rdi\n");
        out.push_str("    movq alya_mem_total_allocs(%rip), %rsi\n");
        out.push_str("    movq alya_mem_total_frees(%rip), %rdx\n");
        out.push_str("    movq alya_mem_active_allocs(%rip), %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    // 3. Bytes
    if is_win {
        out.push_str("    lea alya_mem_fmt_bytes(%rip), %rcx\n");
        out.push_str("    movq alya_mem_total_bytes(%rip), %rdx\n");
        out.push_str("    movq alya_mem_peak_bytes(%rip), %r8\n");
        out.push_str("    movq alya_mem_active_bytes(%rip), %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_bytes(%rip), %rdi\n");
        out.push_str("    movq alya_mem_total_bytes(%rip), %rsi\n");
        out.push_str("    movq alya_mem_peak_bytes(%rip), %rdx\n");
        out.push_str("    movq alya_mem_active_bytes(%rip), %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    // 4. Object Summary
    if is_win {
        out.push_str("    lea alya_mem_fmt_objects(%rip), %rcx\n");
        out.push_str("    movq alya_mem_live_structs(%rip), %rdx\n");
        out.push_str("    movq alya_mem_live_arrays(%rip), %r8\n");
        out.push_str("    movq alya_mem_live_maps(%rip), %r9\n");
        out.push_str("    movq alya_mem_live_strings(%rip), %rax\n");
        out.push_str("    movq %rax, 32(%rsp)\n");
        out.push_str("    movq alya_mem_live_raw(%rip), %rax\n");
        out.push_str("    movq %rax, 40(%rsp)\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_objects(%rip), %rdi\n");
        out.push_str("    movq alya_mem_live_structs(%rip), %rsi\n");
        out.push_str("    movq alya_mem_live_arrays(%rip), %rdx\n");
        out.push_str("    movq alya_mem_live_maps(%rip), %rcx\n");
        out.push_str("    movq alya_mem_live_strings(%rip), %r8\n");
        out.push_str("    movq alya_mem_live_raw(%rip), %r9\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    // 5. Clean vs Leaks
    out.push_str("    movq alya_mem_active_allocs(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_mem_rep_has_leaks\n");

    // Clean execution
    if is_win {
        out.push_str("    lea alya_mem_fmt_clean(%rip), %rcx\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_clean(%rip), %rdi\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }
    out.push_str("    jmp .L_mem_rep_flush\n");

    // Has leaks!
    out.push_str(".L_mem_rep_has_leaks:\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_warn(%rip), %rcx\n");
        out.push_str("    movq alya_mem_active_allocs(%rip), %rdx\n");
        out.push_str("    movq alya_mem_active_bytes(%rip), %r8\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_warn(%rip), %rdi\n");
        out.push_str("    movq alya_mem_active_allocs(%rip), %rsi\n");
        out.push_str("    movq alya_mem_active_bytes(%rip), %rdx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    // Iterate through leaked records in linked list
    out.push_str("    movq alya_mem_records_head(%rip), %rbx\n");
    out.push_str("    movq $1, %r12\n"); // index counter

    out.push_str(".L_mem_rep_loop:\n");
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_mem_rep_footer\n");
    out.push_str("    cmpq $50, %r12\n");
    out.push_str("    jg .L_mem_rep_footer\n");

    out.push_str("    movq 0(%rbx), %r13\n");  // ptr
    out.push_str("    movq 8(%rbx), %r14\n");  // size
    out.push_str("    movq 16(%rbx), %r15\n"); // kind
    out.push_str("    movq 24(%rbx), %rax\n"); // desc

    // Check kind
    out.push_str("    cmpq $3, %r15\n");
    out.push_str("    jne .L_mem_rep_chk_arr\n");

    // Struct: fmt_item_struct(index, ptr, size, desc)
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_mem_rep_desc_ok\n");
    out.push_str("    lea alya_str_anon_struct(%rip), %rax\n");
    out.push_str(".L_mem_rep_desc_ok:\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_item_struct(%rip), %rcx\n");
        out.push_str("    movq %r12, %rdx\n");
        out.push_str("    movq %r13, %r8\n");
        out.push_str("    movq %r14, %r9\n");
        out.push_str("    movq %rax, 32(%rsp)\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    movq %rax, %r8\n");
        out.push_str("    lea alya_mem_fmt_item_struct(%rip), %rdi\n");
        out.push_str("    movq %r12, %rsi\n");
        out.push_str("    movq %r13, %rdx\n");
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }
    out.push_str("    jmp .L_mem_rep_loop_next\n");

    out.push_str(".L_mem_rep_chk_arr:\n");
    out.push_str("    cmpq $1, %r15\n");
    out.push_str("    jne .L_mem_rep_chk_map\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_item_array(%rip), %rcx\n");
        out.push_str("    movq %r12, %rdx\n");
        out.push_str("    movq %r13, %r8\n");
        out.push_str("    movq %r14, %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_item_array(%rip), %rdi\n");
        out.push_str("    movq %r12, %rsi\n");
        out.push_str("    movq %r13, %rdx\n");
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }
    out.push_str("    jmp .L_mem_rep_loop_next\n");

    out.push_str(".L_mem_rep_chk_map:\n");
    out.push_str("    cmpq $2, %r15\n");
    out.push_str("    jne .L_mem_rep_chk_str\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_item_map(%rip), %rcx\n");
        out.push_str("    movq %r12, %rdx\n");
        out.push_str("    movq %r13, %r8\n");
        out.push_str("    movq %r14, %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_item_map(%rip), %rdi\n");
        out.push_str("    movq %r12, %rsi\n");
        out.push_str("    movq %r13, %rdx\n");
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }
    out.push_str("    jmp .L_mem_rep_loop_next\n");

    out.push_str(".L_mem_rep_chk_str:\n");
    out.push_str("    cmpq $4, %r15\n");
    out.push_str("    jne .L_mem_rep_raw\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_item_str(%rip), %rcx\n");
        out.push_str("    movq %r12, %rdx\n");
        out.push_str("    movq %r13, %r8\n");
        out.push_str("    movq %r14, %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_item_str(%rip), %rdi\n");
        out.push_str("    movq %r12, %rsi\n");
        out.push_str("    movq %r13, %rdx\n");
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }
    out.push_str("    jmp .L_mem_rep_loop_next\n");

    out.push_str(".L_mem_rep_raw:\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_item_raw(%rip), %rcx\n");
        out.push_str("    movq %r12, %rdx\n");
        out.push_str("    movq %r13, %r8\n");
        out.push_str("    movq %r14, %r9\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_item_raw(%rip), %rdi\n");
        out.push_str("    movq %r12, %rsi\n");
        out.push_str("    movq %r13, %rdx\n");
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    out.push_str(".L_mem_rep_loop_next:\n");
    out.push_str("    incq %r12\n");
    out.push_str("    movq 32(%rbx), %rbx\n");
    out.push_str("    jmp .L_mem_rep_loop\n");

    out.push_str(".L_mem_rep_footer:\n");
    if is_win {
        out.push_str("    lea alya_mem_fmt_footer(%rip), %rcx\n");
        out.push_str("    call printf\n");
    } else {
        out.push_str("    lea alya_mem_fmt_footer(%rip), %rdi\n");
        out.push_str("    xor %al, %al\n");
        out.push_str(&format!("    call {}printf\n", p));
    }

    out.push_str(".L_mem_rep_flush:\n");
    if is_win {
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call fflush\n");
    } else {
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str(&format!("    call {}fflush\n", p));
    }

    out.push_str("    lea -56(%rbp), %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_mem_rep_ret:\n");
    out.push_str("    ret\n\n");
}
