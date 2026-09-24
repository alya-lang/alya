use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // fn_gc_add_purple(ptr)
    // Marks candidate container as PURPLE (color 3) and records it in roots set.
    // =========================================================================
    out.push_str(".global fn_gc_add_purple\n");
    out.push_str("fn_gc_add_purple:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    mov %rcx, %r11\n");
    } else {
        out.push_str("    mov %rdi, %r11\n");
    }
    out.push_str("    test $7, %r11\n");
    out.push_str("    jnz .L_x64_gc_ap_done\n");
    out.push_str("    cmp $65536, %r11\n");
    out.push_str("    jbe .L_x64_gc_ap_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    ja .L_x64_gc_ap_done\n");
    out.push_str("    movl -16(%r11), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    je .L_x64_gc_ap_valid\n");
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str("    je .L_x64_gc_ap_valid\n");
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_gc_ap_done\n");
    out.push_str(".L_x64_gc_ap_valid:\n");
    out.push_str("    cmpq $0, alya_gc_in_progress(%rip)\n");
    out.push_str("    jne .L_x64_gc_ap_done\n");
    out.push_str("    cmpl $3, -12(%r11)\n");
    out.push_str("    je .L_x64_gc_ap_done\n");
    out.push_str("    movl $3, -12(%r11)\n");
    out.push_str("    movq alya_gc_roots_count(%rip), %rax\n");
    out.push_str("    cmpq $65536, %rax\n");
    out.push_str("    jae .L_x64_gc_ap_done\n");
    out.push_str("    lea alya_gc_roots(%rip), %rdx\n");
    out.push_str("    movq %r11, (%rdx, %rax, 8)\n");
    out.push_str("    incq %rax\n");
    out.push_str("    movq %rax, alya_gc_roots_count(%rip)\n");
    out.push_str("    cmpq $1000000, %rax\n");
    out.push_str("    jb .L_x64_gc_ap_done\n");
    out.push_str("    sub $32, %rsp\n");
    out.push_str("    call fn_gc_collect\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str(".L_x64_gc_ap_done:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_mark_gray(s)
    // Trial-decrements reference counts across cyclic subgraph.
    // =========================================================================
    out.push_str("alya_gc_mark_gray:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test $7, %rbx\n");
    out.push_str("    jnz .L_x64_mg_done\n");
    out.push_str("    cmp $65536, %rbx\n");
    out.push_str("    jbe .L_x64_mg_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    ja .L_x64_mg_done\n");
    out.push_str("    movl -16(%rbx), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    je .L_x64_mg_valid\n");
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str("    je .L_x64_mg_valid\n");
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_mg_done\n");
    out.push_str(".L_x64_mg_valid:\n");
    out.push_str("    cmpl $1, -12(%rbx)\n");
    out.push_str("    je .L_x64_mg_done\n");
    out.push_str("    movl $1, -12(%rbx)\n"); // Color = GREY

    // Traverse children: Struct
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_mg_arr\n");
    out.push_str("    movq (%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_mg_done\n");
    out.push_str("    movq 8(%r12), %r13\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_mg_struct_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_mg_done\n");
    out.push_str("    movq 8(%rbx, %r14, 8), %r15\n");
    emit_child_mark_gray(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_mg_struct_loop\n");

    // Traverse children: Array
    out.push_str(".L_x64_mg_arr:\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    jne .L_x64_mg_map\n");
    out.push_str("    movq (%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_mg_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_mg_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_mg_arr_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_mg_done\n");
    out.push_str("    movq (%r12, %r14, 8), %r15\n");
    emit_child_mark_gray(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_mg_arr_loop\n");

    // Traverse children: Map
    out.push_str(".L_x64_mg_map:\n");
    out.push_str("    movq 8(%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_mg_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_mg_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_mg_map_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_mg_done\n");
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    cmpl $1, 16(%rax)\n");
    out.push_str("    jne .L_x64_mg_map_next\n");
    out.push_str("    movq 0(%rax), %r15\n");
    emit_child_mark_gray(out, is_win);
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    movq 8(%rax), %r15\n");
    emit_child_mark_gray(out, is_win);
    out.push_str(".L_x64_mg_map_next:\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_mg_map_loop\n");

    out.push_str(".L_x64_mg_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan(s)
    // Classifies objects into WHITE (garbage) or revives to BLACK if rc > 0.
    // =========================================================================
    out.push_str("alya_gc_scan:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test $7, %rbx\n");
    out.push_str("    jnz .L_x64_s_done\n");
    out.push_str("    cmp $65536, %rbx\n");
    out.push_str("    jbe .L_x64_s_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    ja .L_x64_s_done\n");
    out.push_str("    cmpl $1, -12(%rbx)\n");
    out.push_str("    jne .L_x64_s_done\n");
    out.push_str("    cmpq $0, -8(%rbx)\n");
    out.push_str("    jle .L_x64_s_to_white\n");

    // External reference alive -> Revive subgraph to BLACK
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
    }
    out.push_str("    call alya_gc_scan_black\n");
    out.push_str("    jmp .L_x64_s_done\n");

    out.push_str(".L_x64_s_to_white:\n");
    out.push_str("    movl $2, -12(%rbx)\n"); // Color = WHITE
    out.push_str("    movl -16(%rbx), %eax\n");

    // Struct children scan
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_s_arr\n");
    out.push_str("    movq (%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_s_done\n");
    out.push_str("    movq 8(%r12), %r13\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_s_struct_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_s_done\n");
    out.push_str("    movq 8(%rbx, %r14, 8), %r15\n");
    emit_child_scan(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_s_struct_loop\n");

    // Array children scan
    out.push_str(".L_x64_s_arr:\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    jne .L_x64_s_map\n");
    out.push_str("    movq (%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_s_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_s_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_s_arr_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_s_done\n");
    out.push_str("    movq (%r12, %r14, 8), %r15\n");
    emit_child_scan(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_s_arr_loop\n");

    // Map children scan
    out.push_str(".L_x64_s_map:\n");
    out.push_str("    movq 8(%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_s_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_s_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_s_map_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_s_done\n");
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    cmpl $1, 16(%rax)\n");
    out.push_str("    jne .L_x64_s_map_next\n");
    out.push_str("    movq 0(%rax), %r15\n");
    emit_child_scan(out, is_win);
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    movq 8(%rax), %r15\n");
    emit_child_scan(out, is_win);
    out.push_str(".L_x64_s_map_next:\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_s_map_loop\n");

    out.push_str(".L_x64_s_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_scan_black(s)
    // Restores trial-decremented reference counts for reachable subgraphs.
    // =========================================================================
    out.push_str("alya_gc_scan_black:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test $7, %rbx\n");
    out.push_str("    jnz .L_x64_sb_done\n");
    out.push_str("    cmp $65536, %rbx\n");
    out.push_str("    jbe .L_x64_sb_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    ja .L_x64_sb_done\n");
    out.push_str("    movl $0, -12(%rbx)\n"); // Color = BLACK
    out.push_str("    movl -16(%rbx), %eax\n");

    // Struct children scan black
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_sb_arr\n");
    out.push_str("    movq (%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_sb_done\n");
    out.push_str("    movq 8(%r12), %r13\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_sb_struct_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_sb_done\n");
    out.push_str("    movq 8(%rbx, %r14, 8), %r15\n");
    emit_child_scan_black(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_sb_struct_loop\n");

    // Array children scan black
    out.push_str(".L_x64_sb_arr:\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    jne .L_x64_sb_map\n");
    out.push_str("    movq (%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_sb_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_sb_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_sb_arr_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_sb_done\n");
    out.push_str("    movq (%r12, %r14, 8), %r15\n");
    emit_child_scan_black(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_sb_arr_loop\n");

    // Map children scan black
    out.push_str(".L_x64_sb_map:\n");
    out.push_str("    movq 8(%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_sb_done\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_sb_done\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_sb_map_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_sb_done\n");
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    cmpl $1, 16(%rax)\n");
    out.push_str("    jne .L_x64_sb_map_next\n");
    out.push_str("    movq 0(%rax), %r15\n");
    emit_child_scan_black(out, is_win);
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    movq 8(%rax), %r15\n");
    emit_child_scan_black(out, is_win);
    out.push_str(".L_x64_sb_map_next:\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_sb_map_loop\n");

    out.push_str(".L_x64_sb_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // alya_gc_collect_white(s)
    // Sweeps confirmed WHITE cyclic objects and frees their heap allocations.
    // =========================================================================
    out.push_str("alya_gc_collect_white:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test $7, %rbx\n");
    out.push_str("    jnz .L_x64_cw_done\n");
    out.push_str("    cmp $65536, %rbx\n");
    out.push_str("    jbe .L_x64_cw_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    ja .L_x64_cw_done\n");
    out.push_str("    cmpl $2, -12(%rbx)\n"); // Must be WHITE (2)
    out.push_str("    jne .L_x64_cw_done\n");
    out.push_str("    movl $0, -12(%rbx)\n"); // Mark BLACK to prevent duplicate sweeping
    out.push_str("    movl -16(%rbx), %eax\n");

    // Struct children collection
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    jne .L_x64_cw_arr\n");
    out.push_str("    movq (%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_cw_free\n");
    out.push_str("    movq 8(%r12), %r13\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_cw_struct_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_cw_free\n");
    out.push_str("    movq 8(%rbx, %r14, 8), %r15\n");
    emit_child_collect_white(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_cw_struct_loop\n");

    // Array children collection
    out.push_str(".L_x64_cw_arr:\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    jne .L_x64_cw_map\n");
    out.push_str("    movq (%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_cw_free\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_cw_free\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_cw_arr_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_cw_free\n");
    out.push_str("    movq (%r12, %r14, 8), %r15\n");
    emit_child_collect_white(out, is_win);
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_cw_arr_loop\n");

    // Map children collection
    out.push_str(".L_x64_cw_map:\n");
    out.push_str("    movq 8(%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_cw_free\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_cw_free\n");
    out.push_str("    xor %r14, %r14\n");
    out.push_str(".L_x64_cw_map_loop:\n");
    out.push_str("    cmp %r13, %r14\n");
    out.push_str("    jge .L_x64_cw_free\n");
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    cmpl $1, 16(%rax)\n");
    out.push_str("    jne .L_x64_cw_map_next\n");
    out.push_str("    movq 0(%rax), %r15\n");
    emit_child_collect_white(out, is_win);
    out.push_str("    lea (%r14, %r14, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    movq 8(%rax), %r15\n");
    emit_child_collect_white(out, is_win);
    out.push_str(".L_x64_cw_map_next:\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_cw_map_loop\n");

    out.push_str(".L_x64_cw_free:\n");
    // Free inner data buffers for Array and Map
    out.push_str("    movl -16(%rbx), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    je .L_x64_cw_free_inner\n");
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str("    jne .L_x64_cw_free_outer\n");
    out.push_str(".L_x64_cw_free_inner:\n");
    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_cw_free_outer\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }

    out.push_str(".L_x64_cw_free_outer:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    lea -16(%rbx), %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    lea -16(%rbx), %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    lock incq alya_gc_collected_cycles(%rip)\n");

    out.push_str(".L_x64_cw_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collect() / fn__gc_collect() -> int
    // Executes full Bacon-Rajan Cycle Collection cycle.
    // Returns number of reclaimed cyclic objects.
    // =========================================================================
    out.push_str(".global fn_gc_collect\n");
    out.push_str("fn_gc_collect:\n");
    out.push_str(".global fn__gc_collect\n");
    out.push_str("fn__gc_collect:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");

    // Reentrancy guard
    out.push_str("    cmpq $0, alya_gc_in_progress(%rip)\n");
    out.push_str("    jne .L_x64_gc_reentrant\n");
    out.push_str("    movq $1, alya_gc_in_progress(%rip)\n");

    out.push_str("    movq alya_gc_roots_count(%rip), %r12\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_gc_early_done\n");

    out.push_str("    movq alya_gc_collected_cycles(%rip), %r15\n");

    // Phase 1: MarkRoots
    out.push_str("    xor %r13, %r13\n");
    out.push_str(".L_x64_gc_p1_loop:\n");
    out.push_str("    cmp %r12, %r13\n");
    out.push_str("    jge .L_x64_gc_p1_done\n");
    out.push_str("    lea alya_gc_roots(%rip), %rax\n");
    out.push_str("    movq (%rax, %r13, 8), %rbx\n");
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_gc_p1_next\n");
    out.push_str("    cmpl $3, -12(%rbx)\n");
    out.push_str("    je .L_x64_gc_p1_call_mg\n");
    out.push_str("    cmpl $1, -12(%rbx)\n");
    out.push_str("    je .L_x64_gc_p1_next\n");
    out.push_str("    movl $0, -12(%rbx)\n");
    out.push_str("    jmp .L_x64_gc_p1_next\n");
    out.push_str(".L_x64_gc_p1_call_mg:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
    }
    out.push_str("    call alya_gc_mark_gray\n");
    out.push_str(".L_x64_gc_p1_next:\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_gc_p1_loop\n");
    out.push_str(".L_x64_gc_p1_done:\n");

    // Phase 2: ScanRoots
    out.push_str("    xor %r13, %r13\n");
    out.push_str(".L_x64_gc_p2_loop:\n");
    out.push_str("    cmp %r12, %r13\n");
    out.push_str("    jge .L_x64_gc_p2_done\n");
    out.push_str("    lea alya_gc_roots(%rip), %rax\n");
    out.push_str("    movq (%rax, %r13, 8), %rbx\n");
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_gc_p2_next\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
    }
    out.push_str("    call alya_gc_scan\n");
    out.push_str(".L_x64_gc_p2_next:\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_gc_p2_loop\n");
    out.push_str(".L_x64_gc_p2_done:\n");

    // Phase 3: CollectRoots
    out.push_str("    xor %r13, %r13\n");
    out.push_str(".L_x64_gc_p3_loop:\n");
    out.push_str("    cmp %r12, %r13\n");
    out.push_str("    jge .L_x64_gc_p3_done\n");
    out.push_str("    lea alya_gc_roots(%rip), %rax\n");
    out.push_str("    movq (%rax, %r13, 8), %rbx\n");
    out.push_str("    movq $0, (%rax, %r13, 8)\n"); // Clear slot
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_gc_p3_next\n");
    out.push_str("    cmpl $2, -12(%rbx)\n");
    out.push_str("    jne .L_x64_gc_p3_next\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
    }
    out.push_str("    call alya_gc_collect_white\n");
    out.push_str(".L_x64_gc_p3_next:\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_gc_p3_loop\n");
    out.push_str(".L_x64_gc_p3_done:\n");

    out.push_str("    movq $0, alya_gc_roots_count(%rip)\n");
    out.push_str("    movq $0, alya_gc_in_progress(%rip)\n");
    out.push_str("    movq alya_gc_collected_cycles(%rip), %rax\n");
    out.push_str("    sub %r15, %rax\n");
    out.push_str("    jmp .L_x64_gc_exit\n");
    out.push_str(".L_x64_gc_early_done:\n");
    out.push_str("    movq $0, alya_gc_in_progress(%rip)\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    jmp .L_x64_gc_exit\n");
    out.push_str(".L_x64_gc_reentrant:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_gc_exit:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collected_count() / fn__gc_collected_count() -> int
    // Returns cumulative count of swept cyclic objects.
    // =========================================================================
    out.push_str(".global fn_gc_collected_count\n");
    out.push_str("fn_gc_collected_count:\n");
    out.push_str(".global fn__gc_collected_count\n");
    out.push_str("fn__gc_collected_count:\n");
    out.push_str("    movq alya_gc_collected_cycles(%rip), %rax\n");
    out.push_str("    ret\n\n");
}

fn emit_child_mark_gray(out: &mut String, is_win: bool) {
    let lbl_next = format!(".L_mg_cn_{:x}", out.len());
    let lbl_cont = format!(".L_mg_cc_{:x}", out.len());
    out.push_str("    test $7, %r15\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %r15\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r15\n");
    out.push_str(&format!("    ja {}\n", lbl_next));
    out.push_str("    movl -16(%r15), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    decq -8(%r15)\n");
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call alya_gc_mark_gray\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan(out: &mut String, is_win: bool) {
    let lbl_next = format!(".L_s_cn_{:x}", out.len());
    let lbl_cont = format!(".L_s_cc_{:x}", out.len());
    out.push_str("    test $7, %r15\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %r15\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r15\n");
    out.push_str(&format!("    ja {}\n", lbl_next));
    out.push_str("    movl -16(%r15), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call alya_gc_scan\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_scan_black(out: &mut String, is_win: bool) {
    let lbl_next = format!(".L_sb_cn_{:x}", out.len());
    let lbl_cont = format!(".L_sb_cc_{:x}", out.len());
    out.push_str("    test $7, %r15\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %r15\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r15\n");
    out.push_str(&format!("    ja {}\n", lbl_next));
    out.push_str("    movl -16(%r15), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str(&format!("    jne {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    incq -8(%r15)\n");
    out.push_str("    cmpl $0, -12(%r15)\n");
    out.push_str(&format!("    je {}\n", lbl_next));
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call alya_gc_scan_black\n");
    out.push_str(&format!("{}:\n", lbl_next));
}

fn emit_child_collect_white(out: &mut String, is_win: bool) {
    let lbl_next = format!(".L_cw_cn_{:x}", out.len());
    let lbl_cont = format!(".L_cw_cc_{:x}", out.len());
    let lbl_str = format!(".L_cw_cs_{:x}", out.len());
    let lbl_ext = format!(".L_cw_ce_{:x}", out.len());
    out.push_str("    test $7, %r15\n");
    out.push_str(&format!("    jnz {}\n", lbl_next));
    out.push_str("    cmp $65536, %r15\n");
    out.push_str(&format!("    jbe {}\n", lbl_next));
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r15\n");
    out.push_str(&format!("    ja {}\n", lbl_next));
    out.push_str("    movl -16(%r15), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_cont));
    out.push_str("    cmp $0x5A110004, %rax\n");
    out.push_str(&format!("    je {}\n", lbl_str));
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_cont));
    out.push_str("    cmpl $2, -12(%r15)\n");
    out.push_str(&format!("    jne {}\n", lbl_ext));
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call alya_gc_collect_white\n");
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_ext));
    out.push_str("    cmpq $0, -8(%r15)\n");
    out.push_str(&format!("    jle {}\n", lbl_next));
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call fn_rc_release\n");
    out.push_str(&format!("    jmp {}\n", lbl_next));
    out.push_str(&format!("{}:\n", lbl_str));
    if is_win {
        out.push_str("    mov %r15, %rcx\n");
    } else {
        out.push_str("    mov %r15, %rdi\n");
    }
    out.push_str("    call fn_rc_release\n");
    out.push_str(&format!("{}:\n", lbl_next));
}
