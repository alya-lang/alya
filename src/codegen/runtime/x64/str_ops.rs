use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_concat
    out.push_str("alya_concat:\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rsi\n");
        out.push_str("    mov %rdx, %r10\n");
    } else {
        out.push_str("    mov %rsi, %r10\n");
        out.push_str("    mov %rdi, %rsi\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_concat_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_concat_ok:\n");
    out.push_str("    lea (%r8, %rbx), %rdi\n");
    out.push_str("    mov %rdi, %rax\n");
    out.push_str("    cmp $65536, %rsi\n");
    out.push_str("    jb .L_x64_copy2_start\n");
    out.push_str(".L_x64_copy1:\n");
    out.push_str("    movb (%rsi), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x64_copy2_start\n");
    out.push_str("    movb %cl, (%rdi)\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    jmp .L_x64_copy1\n");
    out.push_str(".L_x64_copy2_start:\n");
    out.push_str("    mov %r10, %rsi\n");
    out.push_str("    cmp $65536, %rsi\n");
    out.push_str("    jb .L_x64_concat_end\n");
    out.push_str(".L_x64_copy2:\n");
    out.push_str("    movb (%rsi), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x64_concat_end\n");
    out.push_str("    movb %cl, (%rdi)\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    jmp .L_x64_copy2\n");
    out.push_str(".L_x64_concat_end:\n");
    out.push_str("    movb $0, (%rdi)\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    sub %r8, %rdi\n");
    out.push_str("    add $7, %rdi\n");
    out.push_str("    and $-8, %rdi\n");
    out.push_str("    mov %rdi, (%r9)\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    ret\n\n");

    // fn_len
    out.push_str("fn_len:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    xor %rax, %rax\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    cmp $65536, %rcx\n");
        out.push_str("    jb .L_x64_len_end\n");
        out.push_str("    mov $0x00007fffffffffff, %rax\n");
        out.push_str("    cmp %rax, %rcx\n");
        out.push_str("    ja .L_x64_len_zero\n");
        out.push_str("    test $7, %rcx\n");
        out.push_str("    jnz .L_x64_len_loop_start_rcx\n");
        out.push_str("    lea alya_rodata_start(%rip), %r11\n");
        out.push_str("    cmp %r11, %rcx\n");
        out.push_str("    jb .L_x64_len_chk_str_buf_rcx\n");
        out.push_str("    lea alya_rodata_end(%rip), %r10\n");
        out.push_str("    cmp %r10, %rcx\n");
        out.push_str("    jb .L_x64_len_loop_start_rcx\n");
        out.push_str(".L_x64_len_chk_str_buf_rcx:\n");
        out.push_str("    lea alya_str_buf(%rip), %r11\n");
        out.push_str("    cmp %r11, %rcx\n");
        out.push_str("    jb .L_x64_len_chk_tag_rcx\n");
        out.push_str("    lea 67108864(%r11), %r10\n");
        out.push_str("    cmp %r10, %rcx\n");
        out.push_str("    jb .L_x64_len_loop_start_rcx\n");
        out.push_str(".L_x64_len_chk_tag_rcx:\n");
        out.push_str("    movl -16(%rcx), %eax\n");
        out.push_str("    cmp $0x5A110001, %eax\n");
        out.push_str("    je .L_x64_len_obj_rcx\n");
        out.push_str("    cmp $0x5A110002, %eax\n");
        out.push_str("    je .L_x64_len_obj_rcx\n");
        out.push_str(".L_x64_len_loop_start_rcx:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_len_loop:\n");
        out.push_str("    cmpb $0, (%rcx, %rax)\n");
        out.push_str("    je .L_x64_len_end\n");
        out.push_str("    inc %rax\n");
        out.push_str("    jmp .L_x64_len_loop\n");
        out.push_str(".L_x64_len_obj_rcx:\n");
        out.push_str("    movq (%rcx), %rax\n");
        out.push_str("    jmp .L_x64_len_end\n");
    } else {
        out.push_str("    cmp $65536, %rdi\n");
        out.push_str("    jb .L_x64_len_end\n");
        out.push_str("    mov $0x00007fffffffffff, %rax\n");
        out.push_str("    cmp %rax, %rdi\n");
        out.push_str("    ja .L_x64_len_zero\n");
        out.push_str("    test $7, %rdi\n");
        out.push_str("    jnz .L_x64_len_loop_start_rdi\n");
        out.push_str("    lea alya_rodata_start(%rip), %r11\n");
        out.push_str("    cmp %r11, %rdi\n");
        out.push_str("    jb .L_x64_len_chk_str_buf_rdi\n");
        out.push_str("    lea alya_rodata_end(%rip), %r10\n");
        out.push_str("    cmp %r10, %rdi\n");
        out.push_str("    jb .L_x64_len_loop_start_rdi\n");
        out.push_str(".L_x64_len_chk_str_buf_rdi:\n");
        out.push_str("    lea alya_str_buf(%rip), %r11\n");
        out.push_str("    cmp %r11, %rdi\n");
        out.push_str("    jb .L_x64_len_chk_tag_rdi\n");
        out.push_str("    lea 67108864(%r11), %r10\n");
        out.push_str("    cmp %r10, %rdi\n");
        out.push_str("    jb .L_x64_len_loop_start_rdi\n");
        out.push_str(".L_x64_len_chk_tag_rdi:\n");
        out.push_str("    movl -16(%rdi), %eax\n");
        out.push_str("    cmp $0x5A110001, %eax\n");
        out.push_str("    je .L_x64_len_obj_rdi\n");
        out.push_str("    cmp $0x5A110002, %eax\n");
        out.push_str("    je .L_x64_len_obj_rdi\n");
        out.push_str(".L_x64_len_loop_start_rdi:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_len_loop:\n");
        out.push_str("    cmpb $0, (%rdi, %rax)\n");
        out.push_str("    je .L_x64_len_end\n");
        out.push_str("    inc %rax\n");
        out.push_str("    jmp .L_x64_len_loop\n");
        out.push_str(".L_x64_len_obj_rdi:\n");
        out.push_str("    movq (%rdi), %rax\n");
        out.push_str("    jmp .L_x64_len_end\n");
    }
    out.push_str(".L_x64_len_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_len_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_upper
    out.push_str("fn_upper:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_upper_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_upper_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    mov %r12, %r13\n");
    out.push_str(".L_x64_upper_loop:\n");
    out.push_str("    movb (%rdi), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x64_upper_done\n");
    out.push_str("    cmpb $'a', %cl\n");
    out.push_str("    jl .L_x64_upper_store\n");
    out.push_str("    cmpb $'z', %cl\n");
    out.push_str("    jg .L_x64_upper_store\n");
    out.push_str("    subb $32, %cl\n");
    out.push_str(".L_x64_upper_store:\n");
    out.push_str("    movb %cl, (%r13)\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_upper_loop\n");
    out.push_str(".L_x64_upper_done:\n");
    out.push_str("    movb $0, (%r13)\n");
    out.push_str("    inc %r13\n");
    out.push_str("    sub %r8, %r13\n");
    out.push_str("    add $7, %r13\n");
    out.push_str("    and $-8, %r13\n");
    out.push_str("    mov %r13, (%r9)\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_lower
    out.push_str("fn_lower:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_lower_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_lower_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    mov %r12, %r13\n");
    out.push_str(".L_x64_lower_loop:\n");
    out.push_str("    movb (%rdi), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x64_lower_done\n");
    out.push_str("    cmpb $'A', %cl\n");
    out.push_str("    jl .L_x64_lower_store\n");
    out.push_str("    cmpb $'Z', %cl\n");
    out.push_str("    jg .L_x64_lower_store\n");
    out.push_str("    addb $32, %cl\n");
    out.push_str(".L_x64_lower_store:\n");
    out.push_str("    movb %cl, (%r13)\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_lower_loop\n");
    out.push_str(".L_x64_lower_done:\n");
    out.push_str("    movb $0, (%r13)\n");
    out.push_str("    inc %r13\n");
    out.push_str("    sub %r8, %r13\n");
    out.push_str("    add $7, %r13\n");
    out.push_str("    and $-8, %r13\n");
    out.push_str("    mov %r13, (%r9)\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_trim
    out.push_str("fn_trim:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
    }
    out.push_str(".L_x64_trim_lws:\n");
    out.push_str("    movb (%rdi), %al\n");
    out.push_str("    cmpb $32, %al\n");
    out.push_str("    je .L_x64_trim_lws_inc\n");
    out.push_str("    cmpb $9, %al\n");
    out.push_str("    je .L_x64_trim_lws_inc\n");
    out.push_str("    cmpb $10, %al\n");
    out.push_str("    je .L_x64_trim_lws_inc\n");
    out.push_str("    cmpb $13, %al\n");
    out.push_str("    je .L_x64_trim_lws_inc\n");
    out.push_str("    jmp .L_x64_trim_lws_done\n");
    out.push_str(".L_x64_trim_lws_inc:\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    jmp .L_x64_trim_lws\n");
    out.push_str(".L_x64_trim_lws_done:\n");
    out.push_str("    mov %rdi, %r12\n");
    out.push_str(".L_x64_trim_end_loop:\n");
    out.push_str("    cmpb $0, (%r12)\n");
    out.push_str("    je .L_x64_trim_find_end_done\n");
    out.push_str("    inc %r12\n");
    out.push_str("    jmp .L_x64_trim_end_loop\n");
    out.push_str(".L_x64_trim_find_end_done:\n");
    out.push_str(".L_x64_trim_tws:\n");
    out.push_str("    cmp %rdi, %r12\n");
    out.push_str("    jle .L_x64_trim_copy_start\n");
    out.push_str("    movb -1(%r12), %al\n");
    out.push_str("    cmpb $32, %al\n");
    out.push_str("    je .L_x64_trim_tws_dec\n");
    out.push_str("    cmpb $9, %al\n");
    out.push_str("    je .L_x64_trim_tws_dec\n");
    out.push_str("    cmpb $10, %al\n");
    out.push_str("    je .L_x64_trim_tws_dec\n");
    out.push_str("    cmpb $13, %al\n");
    out.push_str("    je .L_x64_trim_tws_dec\n");
    out.push_str("    jmp .L_x64_trim_copy_start\n");
    out.push_str(".L_x64_trim_tws_dec:\n");
    out.push_str("    dec %r12\n");
    out.push_str("    jmp .L_x64_trim_tws\n");
    out.push_str(".L_x64_trim_copy_start:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_trim_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_trim_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r13\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str("    mov %r13, %r14\n");
    out.push_str(".L_x64_trim_copy_loop:\n");
    out.push_str("    cmp %r12, %rdi\n");
    out.push_str("    jge .L_x64_trim_done\n");
    out.push_str("    movb (%rdi), %cl\n");
    out.push_str("    movb %cl, (%r14)\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    inc %r14\n");
    out.push_str("    jmp .L_x64_trim_copy_loop\n");
    out.push_str(".L_x64_trim_done:\n");
    out.push_str("    movb $0, (%r14)\n");
    out.push_str("    inc %r14\n");
    out.push_str("    sub %r8, %r14\n");
    out.push_str("    add $7, %r14\n");
    out.push_str("    and $-8, %r14\n");
    out.push_str("    mov %r14, (%r9)\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_substring / fn_substr
    out.push_str("fn_substr:\n");
    out.push_str("fn_substring:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
        out.push_str("    mov %r8, %rdx\n");
    }
    out.push_str("    cmp $0, %rsi\n");
    out.push_str("    jge .L_x64_sub_adv\n");
    out.push_str("    xor %rsi, %rsi\n");
    out.push_str(".L_x64_sub_adv:\n");
    out.push_str("    test %rsi, %rsi\n");
    out.push_str("    jle .L_x64_sub_adv_done\n");
    out.push_str("    cmpb $0, (%rdi)\n");
    out.push_str("    je .L_x64_sub_adv_done\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    dec %rsi\n");
    out.push_str("    jmp .L_x64_sub_adv\n");
    out.push_str(".L_x64_sub_adv_done:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_sub_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_sub_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    mov %r12, %r13\n");
    out.push_str(".L_x64_sub_copy:\n");
    out.push_str("    cmpb $0, (%rdi)\n");
    out.push_str("    je .L_x64_sub_done\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_sub_done\n");
    out.push_str("    movb (%rdi), %cl\n");
    out.push_str("    movb %cl, (%r13)\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    inc %r13\n");
    out.push_str("    cmp $0, %rdx\n");
    out.push_str("    jl .L_x64_sub_copy\n");
    out.push_str("    dec %rdx\n");
    out.push_str("    jmp .L_x64_sub_copy\n");
    out.push_str(".L_x64_sub_done:\n");
    out.push_str("    movb $0, (%r13)\n");
    out.push_str("    inc %r13\n");
    out.push_str("    sub %r8, %r13\n");
    out.push_str("    add $7, %r13\n");
    out.push_str("    and $-8, %r13\n");
    out.push_str("    mov %r13, (%r9)\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_char_at
    out.push_str("fn_char_at:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
    }
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str("    test %rdi, %rdi\n");
    out.push_str("    jz .L_x64_char_at_end\n");
    out.push_str("    cmp $0, %rsi\n");
    out.push_str("    jl .L_x64_char_at_end\n");
    out.push_str(".L_x64_char_at_adv:\n");
    out.push_str("    test %rsi, %rsi\n");
    out.push_str("    jle .L_x64_char_at_fetch\n");
    out.push_str("    cmpb $0, (%rdi)\n");
    out.push_str("    je .L_x64_char_at_end\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    dec %rsi\n");
    out.push_str("    jmp .L_x64_char_at_adv\n");
    out.push_str(".L_x64_char_at_fetch:\n");
    out.push_str("    movzbl (%rdi), %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x64_char_at_end\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_char_at_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_char_at_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    movb %dl, (%r12)\n");
    out.push_str("    movb $0, 1(%r12)\n");
    out.push_str("    add $2, %r12\n");
    out.push_str("    sub %r8, %r12\n");
    out.push_str("    add $7, %r12\n");
    out.push_str("    and $-8, %r12\n");
    out.push_str("    mov %r12, (%r9)\n");
    out.push_str(".L_x64_char_at_end:\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_ord / fn_char_code
    out.push_str("fn_char_code:\n");
    out.push_str("fn_ord:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_ord_ret\n");
    out.push_str("    cmp $256, %rax\n");
    out.push_str("    jb .L_x64_ord_ret\n");
    out.push_str("    movzbl (%rax), %eax\n");
    out.push_str(".L_x64_ord_ret:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_chr / fn_char_from_code
    out.push_str("fn_char_from_code:\n");
    out.push_str("fn_chr:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdx\n");
    } else {
        out.push_str("    mov %rdi, %rdx\n");
    }
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_chr_end\n");
    out.push_str("    cmp $256, %rdx\n");
    out.push_str("    jb .L_x64_chr_code\n");
    out.push_str("    movzbl (%rdx), %edx\n");
    out.push_str(".L_x64_chr_code:\n");
    out.push_str("    and $255, %edx\n");
    out.push_str("    jz .L_x64_chr_end\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_chr_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_chr_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    movb %dl, (%r12)\n");
    out.push_str("    movb $0, 1(%r12)\n");
    out.push_str("    add $2, %r12\n");
    out.push_str("    sub %r8, %r12\n");
    out.push_str("    add $7, %r12\n");
    out.push_str("    and $-8, %r12\n");
    out.push_str("    mov %r12, (%r9)\n");
    out.push_str(".L_x64_chr_end:\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_from_float
    out.push_str(".global fn_str_from_float\n");
    out.push_str("fn_str_from_float:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r14\n");
    } else {
        out.push_str("    movq %xmm0, %r14\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_str_flt_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_str_flt_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %r13\n");
    out.push_str("    mov %rbx, %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    lea alya_fmt_flt_val(%rip), %rdx\n");
        out.push_str("    mov %r14, %r8\n");
        out.push_str("    movq %r14, %xmm2\n");
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    call sprintf\n");
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str("    lea alya_fmt_flt_val(%rip), %rsi\n");
        out.push_str("    movq %r14, %xmm0\n");
        out.push_str("    mov $1, %rax\n");
        out.push_str("    sub $8, %rsp\n");
        let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
        out.push_str(&format!("    call {}sprintf\n", p));
        out.push_str("    add $8, %rsp\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    add %rax, %r15\n");
    out.push_str("    inc %r15\n");
    out.push_str("    add $7, %r15\n");
    out.push_str("    and $-8, %r15\n");
    out.push_str("    mov %r15, (%r9)\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_from_int
    out.push_str(".global fn_str_from_int\n");
    out.push_str("fn_str_from_int:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    jmp .L_x64_str_convert\n\n");

    // fn_str
    out.push_str(".global fn_str\n");
    out.push_str("fn_str:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    // If argument is already a string in the current thread's alya_str_buf active region, return it directly
    super::emit_str_buf_ctx(out, os);
    out.push_str("    cmp %r8, %rax\n");
    out.push_str("    jb .L_x64_str_chk_rodata\n");
    out.push_str("    mov (%r9), %r10\n");
    out.push_str("    add %r8, %r10\n");
    out.push_str("    cmp %r10, %rax\n");
    out.push_str("    jae .L_x64_str_chk_rodata\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_str_chk_rodata:\n");
    // If argument is already a string literal in .rodata, return it directly
    out.push_str("    lea alya_rodata_start(%rip), %r11\n");
    out.push_str("    cmp %r11, %rax\n");
    out.push_str("    jb .L_x64_str_chk_argv\n");
    out.push_str("    lea alya_rodata_end(%rip), %r10\n");
    out.push_str("    cmp %r10, %rax\n");
    out.push_str("    jae .L_x64_str_chk_argv\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_str_chk_argv:\n");
    out.push_str("    movq alya_argc(%rip), %r10\n");
    out.push_str("    test %r10, %r10\n");
    out.push_str("    jz .L_x64_str_convert\n");
    out.push_str("    movq alya_argv(%rip), %r11\n");
    out.push_str("    test %r11, %r11\n");
    out.push_str("    jz .L_x64_str_convert\n");
    out.push_str("    xor %rcx, %rcx\n");
    out.push_str(".L_x64_str_argv_loop:\n");
    out.push_str("    cmp %r10, %rcx\n");
    out.push_str("    jge .L_x64_str_convert\n");
    out.push_str("    cmpq (%r11, %rcx, 8), %rax\n");
    out.push_str("    je .L_x64_str_ret_direct\n");
    out.push_str("    inc %rcx\n");
    out.push_str("    jmp .L_x64_str_argv_loop\n");
    out.push_str(".L_x64_str_ret_direct:\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_str_convert:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    mov (%r9), %rbx\n");
    out.push_str("    cmp $950000, %rbx\n");
    out.push_str("    jb .L_x64_str_buf_ok\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_str_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r12\n");
    out.push_str("    mov %r12, %r14\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_str_chk_neg\n");
    out.push_str("    movb $'0', (%r12)\n");
    out.push_str("    movb $0, 1(%r12)\n");
    out.push_str("    add $2, %r12\n");
    out.push_str("    jmp .L_x64_str_finish\n");
    out.push_str(".L_x64_str_chk_neg:\n");
    out.push_str("    jns .L_x64_str_pos\n");
    out.push_str("    movb $'-', (%r12)\n");
    out.push_str("    inc %r12\n");
    out.push_str("    neg %rax\n");
    out.push_str(".L_x64_str_pos:\n");
    out.push_str("    xor %r13, %r13\n");
    out.push_str("    mov $10, %rbx\n");
    out.push_str(".L_x64_str_div_loop:\n");
    out.push_str("    xor %rdx, %rdx\n");
    out.push_str("    div %rbx\n");
    out.push_str("    add $'0', %dl\n");
    out.push_str("    push %rdx\n");
    out.push_str("    inc %r13\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_str_div_loop\n");
    out.push_str(".L_x64_str_copy_loop:\n");
    out.push_str("    pop %rdx\n");
    out.push_str("    movb %dl, (%r12)\n");
    out.push_str("    inc %r12\n");
    out.push_str("    dec %r13\n");
    out.push_str("    jnz .L_x64_str_copy_loop\n");
    out.push_str("    movb $0, (%r12)\n");
    out.push_str("    inc %r12\n");
    out.push_str(".L_x64_str_finish:\n");
    out.push_str("    sub %r8, %r12\n");
    out.push_str("    add $7, %r12\n");
    out.push_str("    and $-8, %r12\n");
    out.push_str("    mov %r12, (%r9)\n");
    out.push_str("    mov %r14, %rax\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_is_digit
    out.push_str("fn_is_digit:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdx\n");
    } else {
        out.push_str("    mov %rdi, %rdx\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_is_digit_end\n");
    out.push_str("    cmp $256, %rdx\n");
    out.push_str("    jb .L_x64_is_digit_cmp\n");
    out.push_str("    movzbl (%rdx), %edx\n");
    out.push_str(".L_x64_is_digit_cmp:\n");
    out.push_str("    cmp $'0', %edx\n");
    out.push_str("    jb .L_x64_is_digit_end\n");
    out.push_str("    cmp $'9', %edx\n");
    out.push_str("    ja .L_x64_is_digit_end\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str(".L_x64_is_digit_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_is_alpha
    out.push_str("fn_is_alpha:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdx\n");
    } else {
        out.push_str("    mov %rdi, %rdx\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_is_alpha_end\n");
    out.push_str("    cmp $256, %rdx\n");
    out.push_str("    jb .L_x64_is_alpha_cmp\n");
    out.push_str("    movzbl (%rdx), %edx\n");
    out.push_str(".L_x64_is_alpha_cmp:\n");
    out.push_str("    cmp $'_', %edx\n");
    out.push_str("    je .L_x64_is_alpha_true\n");
    out.push_str("    cmp $'a', %edx\n");
    out.push_str("    jb .L_x64_is_alpha_upper\n");
    out.push_str("    cmp $'z', %edx\n");
    out.push_str("    jbe .L_x64_is_alpha_true\n");
    out.push_str(".L_x64_is_alpha_upper:\n");
    out.push_str("    cmp $'A', %edx\n");
    out.push_str("    jb .L_x64_is_alpha_end\n");
    out.push_str("    cmp $'Z', %edx\n");
    out.push_str("    jbe .L_x64_is_alpha_true\n");
    out.push_str("    jmp .L_x64_is_alpha_end\n");
    out.push_str(".L_x64_is_alpha_true:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str(".L_x64_is_alpha_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_is_alnum
    out.push_str("fn_is_alnum:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_is_alpha\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jnz .L_x64_is_alnum_end\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_is_digit\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    call fn_is_alpha\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jnz .L_x64_is_alnum_end\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    call fn_is_digit\n");
    }
    out.push_str(".L_x64_is_alnum_end:\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_is_space / fn_is_whitespace
    out.push_str("fn_is_whitespace:\n");
    out.push_str("fn_is_space:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdx\n");
    } else {
        out.push_str("    mov %rdi, %rdx\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_is_space_end\n");
    out.push_str("    cmp $256, %rdx\n");
    out.push_str("    jb .L_x64_is_space_cmp\n");
    out.push_str("    movzbl (%rdx), %edx\n");
    out.push_str(".L_x64_is_space_cmp:\n");
    out.push_str("    cmp $' ', %edx\n");
    out.push_str("    je .L_x64_is_space_true\n");
    out.push_str("    cmp $'\\t', %edx\n");
    out.push_str("    je .L_x64_is_space_true\n");
    out.push_str("    cmp $'\\n', %edx\n");
    out.push_str("    je .L_x64_is_space_true\n");
    out.push_str("    cmp $'\\r', %edx\n");
    out.push_str("    je .L_x64_is_space_true\n");
    out.push_str("    jmp .L_x64_is_space_end\n");
    out.push_str(".L_x64_is_space_true:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str(".L_x64_is_space_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_to_int
    out.push_str(".global fn_str_to_int\n");
    out.push_str("fn_str_to_int:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rsi\n");
    } else {
        out.push_str("    mov %rdi, %rsi\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    xor %rcx, %rcx\n");
    out.push_str("    test %rsi, %rsi\n");
    out.push_str("    jz .L_x64_s2i_done\n");
    out.push_str("    lea alya_str_buf(%rip), %r11\n");
    out.push_str("    cmp %r11, %rsi\n");
    out.push_str("    jb .L_x64_s2i_chk_rodata\n");
    out.push_str("    lea 67108864(%r11), %r10\n");
    out.push_str("    cmp %r10, %rsi\n");
    out.push_str("    jb .L_x64_s2i_skip\n");
    out.push_str(".L_x64_s2i_chk_rodata:\n");
    out.push_str("    lea alya_rodata_start(%rip), %r11\n");
    out.push_str("    cmp %r11, %rsi\n");
    out.push_str("    jb .L_x64_s2i_not_str\n");
    out.push_str("    lea alya_rodata_end(%rip), %r10\n");
    out.push_str("    cmp %r10, %rsi\n");
    out.push_str("    jb .L_x64_s2i_skip\n");
    out.push_str(".L_x64_s2i_not_str:\n");
    out.push_str("    mov %rsi, %rdx\n");
    out.push_str("    sar $52, %rdx\n");
    out.push_str("    and $0x7ff, %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_s2i_check_ptr\n");
    out.push_str("    cmp $0x7ff, %rdx\n");
    out.push_str("    je .L_x64_s2i_as_int\n");
    out.push_str("    movq %rsi, %xmm0\n");
    out.push_str("    cvttsd2siq %xmm0, %rax\n");
    out.push_str("    jmp .L_x64_s2i_done\n");
    out.push_str(".L_x64_s2i_check_ptr:\n");
    out.push_str("    cmp $65536, %rsi\n");
    out.push_str("    jb .L_x64_s2i_as_int\n");
    out.push_str("    jmp .L_x64_s2i_skip\n");
    out.push_str(".L_x64_s2i_as_int:\n");
    out.push_str("    mov %rsi, %rax\n");
    out.push_str("    jmp .L_x64_s2i_done\n");
    out.push_str(".L_x64_s2i_skip:\n");
    out.push_str("    movzbq (%rsi), %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_s2i_done\n");
    out.push_str("    cmp $' ', %rdx\n");
    out.push_str("    je .L_x64_s2i_next\n");
    out.push_str("    cmp $'\\t', %rdx\n");
    out.push_str("    je .L_x64_s2i_next\n");
    out.push_str("    cmp $'\\n', %rdx\n");
    out.push_str("    je .L_x64_s2i_next\n");
    out.push_str("    cmp $'\\r', %rdx\n");
    out.push_str("    je .L_x64_s2i_next\n");
    out.push_str("    jmp .L_x64_s2i_sign\n");
    out.push_str(".L_x64_s2i_next:\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    jmp .L_x64_s2i_skip\n");
    out.push_str(".L_x64_s2i_sign:\n");
    out.push_str("    cmp $'-', %rdx\n");
    out.push_str("    jne .L_x64_s2i_check_plus\n");
    out.push_str("    mov $1, %rcx\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    jmp .L_x64_s2i_digits\n");
    out.push_str(".L_x64_s2i_check_plus:\n");
    out.push_str("    cmp $'+', %rdx\n");
    out.push_str("    jne .L_x64_s2i_digits\n");
    out.push_str("    inc %rsi\n");
    out.push_str(".L_x64_s2i_digits:\n");
    out.push_str("    movzbq (%rsi), %rdx\n");
    out.push_str("    sub $'0', %rdx\n");
    out.push_str("    cmp $9, %rdx\n");
    out.push_str("    ja .L_x64_s2i_apply_sign\n");
    out.push_str("    imul $10, %rax\n");
    out.push_str("    add %rdx, %rax\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    jmp .L_x64_s2i_digits\n");
    out.push_str(".L_x64_s2i_apply_sign:\n");
    out.push_str("    test %rcx, %rcx\n");
    out.push_str("    jz .L_x64_s2i_done\n");
    out.push_str("    neg %rax\n");
    out.push_str(".L_x64_s2i_done:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_to_float
    out.push_str(".global fn_str_to_float\n");
    out.push_str("fn_str_to_float:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rsi\n");
    } else {
        out.push_str("    mov %rdi, %rsi\n");
    }
    out.push_str("    test %rsi, %rsi\n");
    out.push_str("    jz .L_x64_s2f_zero\n");
    out.push_str("    lea alya_str_buf(%rip), %r11\n");
    out.push_str("    cmp %r11, %rsi\n");
    out.push_str("    jb .L_x64_s2f_chk_rodata\n");
    out.push_str("    lea 67108864(%r11), %r10\n");
    out.push_str("    cmp %r10, %rsi\n");
    out.push_str("    jb .L_x64_s2f_call_atof\n");
    out.push_str(".L_x64_s2f_chk_rodata:\n");
    out.push_str("    lea alya_rodata_start(%rip), %r11\n");
    out.push_str("    cmp %r11, %rsi\n");
    out.push_str("    jb .L_x64_s2f_not_str\n");
    out.push_str("    lea alya_rodata_end(%rip), %r10\n");
    out.push_str("    cmp %r10, %rsi\n");
    out.push_str("    jb .L_x64_s2f_call_atof\n");
    out.push_str(".L_x64_s2f_not_str:\n");
    out.push_str("    mov %rsi, %rdx\n");
    out.push_str("    sar $52, %rdx\n");
    out.push_str("    and $0x7ff, %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_s2f_check_ptr\n");
    out.push_str("    cmp $0x7ff, %rdx\n");
    out.push_str("    je .L_x64_s2f_from_int\n");
    out.push_str("    mov %rsi, %rax\n");
    out.push_str("    jmp .L_x64_s2f_end\n");
    out.push_str(".L_x64_s2f_check_ptr:\n");
    out.push_str("    cmp $65536, %rsi\n");
    out.push_str("    jb .L_x64_s2f_from_int\n");
    out.push_str("    jmp .L_x64_s2f_call_atof\n");
    out.push_str(".L_x64_s2f_from_int:\n");
    out.push_str("    cvtsi2sdq %rsi, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    jmp .L_x64_s2f_end\n");
    out.push_str(".L_x64_s2f_call_atof:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rsi, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call atof\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rsi, %rdi\n");
        out.push_str(&format!("    call {}atof\n", p));
    }
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    jmp .L_x64_s2f_end\n");
    out.push_str(".L_x64_s2f_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_s2f_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_char_count
    out.push_str(".global fn_char_count\n");
    out.push_str("fn_char_count:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    xor %rax, %rax\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rsi\n");
    } else {
        out.push_str("    mov %rdi, %rsi\n");
    }
    out.push_str("    test %rsi, %rsi\n");
    out.push_str("    jz .L_x64_char_count_done\n");
    out.push_str(".L_x64_char_count_loop:\n");
    out.push_str("    movzbq (%rsi), %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_char_count_done\n");
    out.push_str("    inc %rsi\n");
    out.push_str("    and $0xC0, %rdx\n");
    out.push_str("    cmp $0x80, %rdx\n");
    out.push_str("    je .L_x64_char_count_loop\n");
    out.push_str("    inc %rax\n");
    out.push_str("    jmp .L_x64_char_count_loop\n");
    out.push_str(".L_x64_char_count_done:\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_bytes
    out.push_str(".global fn_bytes\n");
    out.push_str("fn_bytes:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
    }
    out.push_str("    xor %r13, %r13\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_bytes_alloc\n");
    out.push_str(".L_x64_bytes_len_loop:\n");
    out.push_str("    cmpb $0, (%r12, %r13)\n");
    out.push_str("    je .L_x64_bytes_alloc\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_bytes_len_loop\n");
    out.push_str(".L_x64_bytes_alloc:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_array_new\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    mov %rax, %r14\n");
    out.push_str("    mov 16(%rax), %r15\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_bytes_copy_loop:\n");
    out.push_str("    cmp %r13, %rbx\n");
    out.push_str("    jge .L_x64_bytes_done\n");
    out.push_str("    movzbq (%r12, %rbx), %rax\n");
    out.push_str("    movq %rax, (%r15, %rbx, 8)\n");
    out.push_str("    inc %rbx\n");
    out.push_str("    jmp .L_x64_bytes_copy_loop\n");
    out.push_str(".L_x64_bytes_done:\n");
    out.push_str("    mov %r14, %rax\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_format_binary
    out.push_str(".global fn_format_binary\n");
    out.push_str("fn_format_binary:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
    }
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rbx\n");
    out.push_str("    cmpq $950000, %rbx\n");
    out.push_str("    jb .L_x64_bin_buf_ok\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_bin_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %r13\n");
    out.push_str("    movb $'0', (%r13)\n");
    out.push_str("    movb $'b', 1(%r13)\n");
    out.push_str("    mov $2, %r14\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jnz .L_x64_bin_nonzero\n");
    out.push_str("    movb $'0', 2(%r13)\n");
    out.push_str("    movb $0, 3(%r13)\n");
    out.push_str("    add $4, %rbx\n");
    out.push_str("    add $7, %rbx\n");
    out.push_str("    and $-8, %rbx\n");
    out.push_str("    movq %rbx, (%r9)\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str("    jmp .L_x64_bin_done\n");
    out.push_str(".L_x64_bin_nonzero:\n");
    out.push_str("    bsrq %r12, %rcx\n");
    out.push_str(".L_x64_bin_loop:\n");
    out.push_str("    btq %rcx, %r12\n");
    out.push_str("    jc .L_x64_bin_bit1\n");
    out.push_str("    movb $'0', (%r13, %r14)\n");
    out.push_str("    jmp .L_x64_bin_bit_next\n");
    out.push_str(".L_x64_bin_bit1:\n");
    out.push_str("    movb $'1', (%r13, %r14)\n");
    out.push_str(".L_x64_bin_bit_next:\n");
    out.push_str("    inc %r14\n");
    out.push_str("    dec %rcx\n");
    out.push_str("    cmp $0, %rcx\n");
    out.push_str("    jge .L_x64_bin_loop\n");
    out.push_str("    movb $0, (%r13, %r14)\n");
    out.push_str("    inc %r14\n");
    out.push_str("    add %r14, %rbx\n");
    out.push_str("    add $7, %rbx\n");
    out.push_str("    and $-8, %rbx\n");
    out.push_str("    movq %rbx, (%r9)\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str(".L_x64_bin_done:\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_runes
    out.push_str(".global fn_runes\n");
    out.push_str("fn_runes:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $40, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    call fn_char_count\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    call fn_char_count\n");
    }
    out.push_str("    mov %rax, %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_array_new\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    mov %rax, %r14\n");
    out.push_str("    mov 16(%rax), %r15\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_runes_loop:\n");
    out.push_str("    cmp %r13, %rbx\n");
    out.push_str("    jge .L_x64_runes_done\n");
    out.push_str("    movzbq (%r12), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_runes_done\n");
    out.push_str("    mov $1, %rcx\n");
    out.push_str("    cmp $0x80, %rax\n");
    out.push_str("    jb .L_x64_runes_len_ok\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    and $0xE0, %rdx\n");
    out.push_str("    cmp $0xC0, %rdx\n");
    out.push_str("    jne .L_x64_runes_chk3\n");
    out.push_str("    mov $2, %rcx\n");
    out.push_str("    jmp .L_x64_runes_len_ok\n");
    out.push_str(".L_x64_runes_chk3:\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    and $0xF0, %rdx\n");
    out.push_str("    cmp $0xE0, %rdx\n");
    out.push_str("    jne .L_x64_runes_chk4\n");
    out.push_str("    mov $3, %rcx\n");
    out.push_str("    jmp .L_x64_runes_len_ok\n");
    out.push_str(".L_x64_runes_chk4:\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    and $0xF8, %rdx\n");
    out.push_str("    cmp $0xF0, %rdx\n");
    out.push_str("    jne .L_x64_runes_len_ok\n");
    out.push_str("    mov $4, %rcx\n");
    out.push_str(".L_x64_runes_len_ok:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rdx\n");
    out.push_str("    cmpq $950000, %rdx\n");
    out.push_str("    jb .L_x64_runes_buf_ok\n");
    out.push_str("    xorq %rdx, %rdx\n");
    out.push_str(".L_x64_runes_buf_ok:\n");
    out.push_str("    lea (%r8, %rdx), %r10\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_runes_copy_byte:\n");
    out.push_str("    cmp %rcx, %rax\n");
    out.push_str("    jge .L_x64_runes_copy_done\n");
    out.push_str("    movb (%r12, %rax), %sil\n");
    out.push_str("    movb %sil, (%r10, %rax)\n");
    out.push_str("    inc %rax\n");
    out.push_str("    jmp .L_x64_runes_copy_byte\n");
    out.push_str(".L_x64_runes_copy_done:\n");
    out.push_str("    movb $0, (%r10, %rcx)\n");
    out.push_str("    inc %rcx\n");
    out.push_str("    add %rcx, %rdx\n");
    out.push_str("    add $7, %rdx\n");
    out.push_str("    and $-8, %rdx\n");
    out.push_str("    movq %rdx, (%r9)\n");
    out.push_str("    movq %r10, (%r15, %rbx, 8)\n");
    out.push_str("    dec %rcx\n");
    out.push_str("    add %rcx, %r12\n");
    out.push_str("    inc %rbx\n");
    out.push_str("    jmp .L_x64_runes_loop\n");
    out.push_str(".L_x64_runes_done:\n");
    out.push_str("    mov %r14, %rax\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
