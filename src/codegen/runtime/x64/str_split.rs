use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_contains
    out.push_str("fn_contains:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
    }
    out.push_str("    cmpb $0, (%rsi)\n");
    out.push_str("    je .L_x64_contains_match\n");
    out.push_str(".L_x64_contains_outer:\n");
    out.push_str("    cmpb $0, (%rdi)\n");
    out.push_str("    je .L_x64_contains_nomatch\n");
    out.push_str("    mov %rdi, %rax\n");
    out.push_str("    mov %rsi, %rdx\n");
    out.push_str(".L_x64_contains_inner:\n");
    out.push_str("    movb (%rdx), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x64_contains_match\n");
    out.push_str("    movb (%rax), %r8b\n");
    out.push_str("    cmp %r8b, %cl\n");
    out.push_str("    jne .L_x64_contains_next\n");
    out.push_str("    inc %rax\n");
    out.push_str("    inc %rdx\n");
    out.push_str("    jmp .L_x64_contains_inner\n");
    out.push_str(".L_x64_contains_next:\n");
    out.push_str("    inc %rdi\n");
    out.push_str("    jmp .L_x64_contains_outer\n");
    out.push_str(".L_x64_contains_match:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_contains_end\n");
    out.push_str(".L_x64_contains_nomatch:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_contains_end:\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_join
    out.push_str(".global fn_join\n");
    out.push_str("fn_join:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $24, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %rcx, %r12\n"); // arr
        out.push_str("    movq %rdx, %r13\n"); // delim
    } else {
        out.push_str("    movq %rdi, %r12\n");
        out.push_str("    movq %rsi, %r13\n");
    }
    out.push_str("    testq %r12, %r12\n");
    out.push_str("    jz .L_x64_join_empty\n");
    out.push_str("    movq (%r12), %rax\n"); // arr->len
    out.push_str("    testq %rax, %rax\n");
    out.push_str("    jz .L_x64_join_empty\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rbx\n");
    out.push_str("    cmpq $950000, %rbx\n");
    out.push_str("    jb .L_x64_join_buf_ok\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_join_buf_ok:\n");
    out.push_str("    lea (%r8, %rbx), %rdi\n");
    out.push_str("    movq %rdi, %r14\n"); // r14 = start_ptr
    out.push_str("    xorq %r15, %r15\n"); // r15 = i (0)
    out.push_str(".L_x64_join_loop:\n");
    out.push_str("    cmpq (%r12), %r15\n");
    out.push_str("    jge .L_x64_join_finish\n");
    out.push_str("    testq %r15, %r15\n");
    out.push_str("    jz .L_x64_join_copy_elem\n");
    out.push_str("    movq %r13, %rsi\n"); // copy delim
    out.push_str(".L_x64_join_copy_delim:\n");
    out.push_str("    movb (%rsi), %al\n");
    out.push_str("    testb %al, %al\n");
    out.push_str("    jz .L_x64_join_copy_elem\n");
    out.push_str("    movb %al, (%rdi)\n");
    out.push_str("    incq %rsi\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    jmp .L_x64_join_copy_delim\n");
    out.push_str(".L_x64_join_copy_elem:\n");
    out.push_str("    movq 16(%r12), %rsi\n"); // arr->data
    out.push_str("    movq (%rsi, %r15, 8), %rsi\n"); // arr->data[i]
    out.push_str(".L_x64_join_copy_elem_loop:\n");
    out.push_str("    movb (%rsi), %al\n");
    out.push_str("    testb %al, %al\n");
    out.push_str("    jz .L_x64_join_elem_done\n");
    out.push_str("    movb %al, (%rdi)\n");
    out.push_str("    incq %rsi\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    jmp .L_x64_join_copy_elem_loop\n");
    out.push_str(".L_x64_join_elem_done:\n");
    out.push_str("    incq %r15\n");
    out.push_str("    jmp .L_x64_join_loop\n");
    out.push_str(".L_x64_join_finish:\n");
    out.push_str("    movb $0, (%rdi)\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    subq %r8, %rdi\n");
    out.push_str("    addq $7, %rdi\n");
    out.push_str("    andq $-8, %rdi\n");
    out.push_str("    movq %rdi, (%r9)\n");
    out.push_str("    movq %r14, %rax\n");
    out.push_str("    jmp .L_x64_join_ret\n");
    out.push_str(".L_x64_join_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_join_ret:\n");
    out.push_str("    addq $24, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_split
    out.push_str(".global fn_split\n");
    out.push_str("fn_split:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rsi\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %rcx, %r12\n"); // str
        out.push_str("    movq %rdx, %r13\n"); // delim
        out.push_str("    xorq %rcx, %rcx\n");
        out.push_str("    call alya_array_new\n");
    } else {
        out.push_str("    movq %rdi, %r12\n");
        out.push_str("    movq %rsi, %r13\n");
        out.push_str("    xorq %rdi, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    movq %rax, %r14\n"); // r14 = arr
    out.push_str("    movq %r13, %rsi\n");
    out.push_str("    xorq %r15, %r15\n");
    out.push_str(".L_x64_split_dlen_loop:\n");
    out.push_str("    cmpb $0, (%rsi, %r15)\n");
    out.push_str("    je .L_x64_split_dlen_done\n");
    out.push_str("    incq %r15\n");
    out.push_str("    jmp .L_x64_split_dlen_loop\n");
    out.push_str(".L_x64_split_dlen_done:\n");
    out.push_str("    testq %r15, %r15\n");
    out.push_str("    jnz .L_x64_split_non_empty_delim\n");
    out.push_str("    movq %r12, 32(%rsp)\n");
    out.push_str(".L_x64_split_empty_loop:\n");
    out.push_str("    movq 32(%rsp), %rsi\n");
    out.push_str("    movb (%rsi), %al\n");
    out.push_str("    testb %al, %al\n");
    out.push_str("    jz .L_x64_split_ret\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rbx\n");
    out.push_str("    cmpq $950000, %rbx\n");
    out.push_str("    jb .L_x64_se1\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_se1:\n");
    out.push_str("    lea (%r8, %rbx), %rdi\n");
    out.push_str("    movq %rdi, %r10\n");
    out.push_str("    movb %al, (%rdi)\n");
    out.push_str("    movb $0, 1(%rdi)\n");
    out.push_str("    addq $2, %rdi\n");
    out.push_str("    subq %r8, %rdi\n");
    out.push_str("    addq $7, %rdi\n");
    out.push_str("    andq $-8, %rdi\n");
    out.push_str("    movq %rdi, (%r9)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    movq %r10, %rdx\n");
        out.push_str("    mov $3, %r8\n");
        out.push_str("    call alya_array_push\n");
    } else {
        out.push_str("    movq %r14, %rdi\n");
        out.push_str("    movq %r10, %rsi\n");
        out.push_str("    mov $3, %rdx\n");
        out.push_str("    call alya_array_push\n");
    }
    out.push_str("    incq 32(%rsp)\n");
    out.push_str("    jmp .L_x64_split_empty_loop\n");
    out.push_str(".L_x64_split_non_empty_delim:\n");
    out.push_str("    movq %r12, 32(%rsp)\n"); // curr = str
    out.push_str("    movq %r12, 40(%rsp)\n"); // token_start = str
    out.push_str(".L_x64_split_main_loop:\n");
    out.push_str("    movq 32(%rsp), %rsi\n");
    out.push_str("    cmpb $0, (%rsi)\n");
    out.push_str("    je .L_x64_split_emit_final\n");
    out.push_str("    movq %r13, %rdx\n"); // delim
    out.push_str("    xorq %rbx, %rbx\n"); // k = 0
    out.push_str(".L_x64_split_cmp_loop:\n");
    out.push_str("    cmpq %r15, %rbx\n");
    out.push_str("    jge .L_x64_split_matched\n");
    out.push_str("    movb (%rdx, %rbx), %al\n");
    out.push_str("    cmpb %al, (%rsi, %rbx)\n");
    out.push_str("    jne .L_x64_split_cmp_fail\n");
    out.push_str("    incq %rbx\n");
    out.push_str("    jmp .L_x64_split_cmp_loop\n");
    out.push_str(".L_x64_split_cmp_fail:\n");
    out.push_str("    incq 32(%rsp)\n");
    out.push_str("    jmp .L_x64_split_main_loop\n");
    out.push_str(".L_x64_split_matched:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rbx\n");
    out.push_str("    cmpq $950000, %rbx\n");
    out.push_str("    jb .L_x64_se2\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_se2:\n");
    out.push_str("    lea (%r8, %rbx), %rdi\n");
    out.push_str("    movq %rdi, 48(%rsp)\n"); // token ptr
    out.push_str("    movq 40(%rsp), %rsi\n"); // src = token_start
    out.push_str("    movq 32(%rsp), %rdx\n"); // end = curr
    out.push_str(".L_x64_split_tok_copy:\n");
    out.push_str("    cmpq %rdx, %rsi\n");
    out.push_str("    jge .L_x64_split_tok_done\n");
    out.push_str("    movb (%rsi), %al\n");
    out.push_str("    movb %al, (%rdi)\n");
    out.push_str("    incq %rsi\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    jmp .L_x64_split_tok_copy\n");
    out.push_str(".L_x64_split_tok_done:\n");
    out.push_str("    movb $0, (%rdi)\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    subq %r8, %rdi\n");
    out.push_str("    addq $7, %rdi\n");
    out.push_str("    andq $-8, %rdi\n");
    out.push_str("    movq %rdi, (%r9)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    movq 48(%rsp), %rdx\n");
        out.push_str("    mov $3, %r8\n");
        out.push_str("    call alya_array_push\n");
    } else {
        out.push_str("    movq %r14, %rdi\n");
        out.push_str("    movq 48(%rsp), %rsi\n");
        out.push_str("    mov $3, %rdx\n");
        out.push_str("    call alya_array_push\n");
    }
    out.push_str("    addq %r15, 32(%rsp)\n"); // curr += delim_len
    out.push_str("    movq 32(%rsp), %rax\n");
    out.push_str("    movq %rax, 40(%rsp)\n"); // token_start = curr
    out.push_str("    jmp .L_x64_split_main_loop\n");
    out.push_str(".L_x64_split_emit_final:\n");
    super::emit_str_buf_ctx(out, os);
    out.push_str("    movq (%r9), %rbx\n");
    out.push_str("    cmpq $950000, %rbx\n");
    out.push_str("    jb .L_x64_se3\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_se3:\n");
    out.push_str("    lea (%r8, %rbx), %rdi\n");
    out.push_str("    movq %rdi, 48(%rsp)\n");
    out.push_str("    movq 40(%rsp), %rsi\n"); // token_start
    out.push_str("    movq 32(%rsp), %rdx\n"); // curr
    out.push_str(".L_x64_split_final_copy:\n");
    out.push_str("    cmpq %rdx, %rsi\n");
    out.push_str("    jge .L_x64_split_final_done\n");
    out.push_str("    movb (%rsi), %al\n");
    out.push_str("    movb %al, (%rdi)\n");
    out.push_str("    incq %rsi\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    jmp .L_x64_split_final_copy\n");
    out.push_str(".L_x64_split_final_done:\n");
    out.push_str("    movb $0, (%rdi)\n");
    out.push_str("    incq %rdi\n");
    out.push_str("    subq %r8, %rdi\n");
    out.push_str("    addq $7, %rdi\n");
    out.push_str("    andq $-8, %rdi\n");
    out.push_str("    movq %rdi, (%r9)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %r14, %rcx\n");
        out.push_str("    movq 48(%rsp), %rdx\n");
        out.push_str("    mov $3, %r8\n");
        out.push_str("    call alya_array_push\n");
    } else {
        out.push_str("    movq %r14, %rdi\n");
        out.push_str("    movq 48(%rsp), %rsi\n");
        out.push_str("    mov $3, %rdx\n");
        out.push_str("    call alya_array_push\n");
    }
    out.push_str(".L_x64_split_ret:\n");
    out.push_str("    movq %r14, %rax\n");
    out.push_str("    addq $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

}
