use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_array_new
    out.push_str("alya_array_new:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
    }
    out.push_str("    mov %r12, %r13\n");
    out.push_str("    cmp $8, %r13\n");
    out.push_str("    jge .L_x64_new_cap_ok\n");
    out.push_str("    mov $8, %r13\n");
    out.push_str("    jmp .L_x64_new_alloc\n");
    out.push_str(".L_x64_new_cap_ok:\n");
    out.push_str("    shl $1, %r13\n");
    out.push_str(".L_x64_new_alloc:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    mov $40, %rdx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call calloc\n");
        out.push_str("    movq $0x5A110001, (%rax)\n");
        out.push_str("    movq $1, 8(%rax)\n");
        out.push_str("    lea 16(%rax), %r14\n");
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    mov $8, %rdx\n");
        out.push_str("    call calloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov $1, %rdi\n");
        out.push_str("    mov $40, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
        out.push_str("    movq $0x5A110001, (%rax)\n");
        out.push_str("    movq $1, 8(%rax)\n");
        out.push_str("    lea 16(%rax), %r14\n");
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    mov $8, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    mov %r13, %r11\n");
    out.push_str("    shl $3, %r11\n");
    out.push_str("    add $40, %r11\n");
    out.push_str("    add %r11, alya_allocated_bytes(%rip)\n");
    out.push_str("    mov %r12, (%r14)\n");
    out.push_str("    mov %r13, 8(%r14)\n");
    out.push_str("    mov %rax, 16(%r14)\n");
    out.push_str("    mov %r14, %rax\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_array_push
    out.push_str("alya_array_push:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    mov (%r12), %r14\n");
    out.push_str("    mov 8(%r12), %r15\n");
    out.push_str("    cmp %r15, %r14\n");
    out.push_str("    jl .L_x64_push_store\n");
    out.push_str("    test %r15, %r15\n");
    out.push_str("    jnz .L_x64_push_double\n");
    out.push_str("    mov $8, %r15\n");
    out.push_str("    jmp .L_x64_push_realloc\n");
    out.push_str(".L_x64_push_double:\n");
    out.push_str("    shl $1, %r15\n");
    out.push_str(".L_x64_push_realloc:\n");
    out.push_str("    mov %r15, 8(%r12)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov 16(%r12), %rcx\n");
        out.push_str("    lea (, %r15, 8), %rdx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call realloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov 16(%r12), %rdi\n");
        out.push_str("    lea (, %r15, 8), %rsi\n");
        out.push_str(&format!("    call {}realloc\n", p));
    }
    out.push_str("    mov %rax, 16(%r12)\n");
    out.push_str("    mov %r15, %r11\n");
    out.push_str("    shl $3, %r11\n");
    out.push_str("    add %r11, alya_allocated_bytes(%rip)\n");
    out.push_str(".L_x64_push_store:\n");
    out.push_str("    mov 16(%r12), %rdx\n");
    out.push_str("    mov (%r12), %rax\n");
    out.push_str("    mov %r13, (%rdx, %rax, 8)\n");
    out.push_str("    inc %rax\n");
    out.push_str("    mov %rax, (%r12)\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_array_pop
    out.push_str("alya_array_pop:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    mov (%rax), %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jle alya_error_index_out_of_bounds\n");
    out.push_str("    dec %rdx\n");
    out.push_str("    mov %rdx, (%rax)\n");
    out.push_str("    mov 16(%rax), %rcx\n");
    out.push_str("    mov (%rcx, %rdx, 8), %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_print_array
    out.push_str("alya_print_array:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jnz .L_x64_arr_not_null\n");
        out.push_str("    lea alya_fmt_arr_empty(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    jmp .L_x64_arr_exit\n");
        out.push_str(".L_x64_arr_not_null:\n");
        out.push_str("    mov (%r12), %r13\n");
        out.push_str("    lea alya_fmt_arr_open(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    xor %r14, %r14\n");
        out.push_str(".L_x64_arr_loop:\n");
        out.push_str("    cmp %r13, %r14\n");
        out.push_str("    jge .L_x64_arr_close_call\n");
        out.push_str("    test %r14, %r14\n");
        out.push_str("    jz .L_x64_arr_print_elem\n");
        out.push_str("    lea alya_fmt_arr_comma(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_arr_print_elem:\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rcx\n");
        out.push_str("    mov 16(%r12), %rax\n");
        out.push_str("    mov (%rax, %r14, 8), %rdx\n");
        out.push_str("    call printf\n");
        out.push_str("    inc %r14\n");
        out.push_str("    jmp .L_x64_arr_loop\n");
        out.push_str(".L_x64_arr_close_call:\n");
        out.push_str("    lea alya_fmt_arr_close(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_arr_exit:\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jnz .L_x64_arr_not_null\n");
        out.push_str("    lea alya_fmt_arr_empty(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    jmp .L_x64_arr_exit\n");
        out.push_str(".L_x64_arr_not_null:\n");
        out.push_str("    mov (%r12), %r13\n");
        out.push_str("    lea alya_fmt_arr_open(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    xor %r14, %r14\n");
        out.push_str(".L_x64_arr_loop:\n");
        out.push_str("    cmp %r13, %r14\n");
        out.push_str("    jge .L_x64_arr_close_call\n");
        out.push_str("    test %r14, %r14\n");
        out.push_str("    jz .L_x64_arr_print_elem\n");
        out.push_str("    lea alya_fmt_arr_comma(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_arr_print_elem:\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rdi\n");
        out.push_str("    mov 16(%r12), %rax\n");
        out.push_str("    mov (%rax, %r14, 8), %rsi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    inc %r14\n");
        out.push_str("    jmp .L_x64_arr_loop\n");
        out.push_str(".L_x64_arr_close_call:\n");
        out.push_str("    lea alya_fmt_arr_close(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_arr_exit:\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_args
    out.push_str(".global fn_args\n");
    out.push_str("fn_args:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    movq alya_argc(%rip), %rax\n");
    out.push_str("    cmpq $1, %rax\n");
    out.push_str("    jg .L_x64_args_has_items\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    xorq %rcx, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_array_new\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    xorq %rdi, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    jmp .L_x64_args_ret\n");
    out.push_str(".L_x64_args_has_items:\n");
    out.push_str("    decq %rax\n");
    out.push_str("    movq %rax, %r12\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movq %r12, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_array_new\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    movq %r12, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    movq %rax, %r13\n");
    out.push_str("    movq alya_argv(%rip), %r14\n");
    out.push_str("    xorq %rbx, %rbx\n");
    out.push_str(".L_x64_args_loop:\n");
    out.push_str("    cmpq %r12, %rbx\n");
    out.push_str("    jge .L_x64_args_done\n");
    out.push_str("    movq 8(%r14, %rbx, 8), %rax\n");
    out.push_str("    movq 16(%r13), %rdx\n");
    out.push_str("    movq %rax, (%rdx, %rbx, 8)\n");
    out.push_str("    incq %rbx\n");
    out.push_str("    jmp .L_x64_args_loop\n");
    out.push_str(".L_x64_args_done:\n");
    out.push_str("    movq %r13, %rax\n");
    out.push_str(".L_x64_args_ret:\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

}
