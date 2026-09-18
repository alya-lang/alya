use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_struct_new
    out.push_str("alya_struct_new:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    lea 3(%r13), %rcx\n");
        out.push_str("    mov $8, %rdx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call calloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
        out.push_str("    lea 3(%r13), %rdi\n");
        out.push_str("    mov $8, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    mov %r13, %r11\n");
    out.push_str("    add $3, %r11\n");
    out.push_str("    shl $3, %r11\n");
    out.push_str("    add %r11, alya_allocated_bytes(%rip)\n");
    out.push_str("    movq $0x5A110003, (%rax)\n");
    out.push_str("    movq $1, 8(%rax)\n");
    out.push_str("    lea 16(%rax), %rax\n");
    out.push_str("    mov %r12, (%rax)\n");
    out.push_str("    push %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov %r11, %rdx\n");
        out.push_str("    mov $3, %r8\n");
        out.push_str("    mov (%r12), %r9\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_alloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov %r11, %rsi\n");
        out.push_str("    mov $3, %rdx\n");
        out.push_str("    mov (%r12), %rcx\n");
        out.push_str("    call alya_mem_track_alloc\n");
    }
    out.push_str("    pop %rax\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_fat_ptr_new
    out.push_str(".global alya_fat_ptr_new\n");
    out.push_str("alya_fat_ptr_new:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if is_win {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    mov $4, %rcx\n");
        out.push_str("    mov $8, %rdx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call calloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
        out.push_str("    mov $4, %rdi\n");
        out.push_str("    mov $8, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    add $32, alya_allocated_bytes(%rip)\n");
    out.push_str("    movq $0x5A110004, (%rax)\n");
    out.push_str("    movq $1, 8(%rax)\n");
    out.push_str("    lea 16(%rax), %rax\n");
    out.push_str("    mov %r12, (%rax)\n");
    out.push_str("    mov %r13, 8(%rax)\n");
    out.push_str("    push %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov $32, %rdx\n");
        out.push_str("    mov $3, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_alloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov $32, %rsi\n");
        out.push_str("    mov $3, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_mem_track_alloc\n");
    }
    out.push_str("    pop %rax\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_print_struct
    out.push_str("alya_print_struct:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jnz .L_x64_struct_not_null\n");
        out.push_str("    lea alya_fmt_struct_null(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    jmp .L_x64_struct_exit\n");
        out.push_str(".L_x64_struct_not_null:\n");
        out.push_str("    mov (%r12), %r13\n");
        out.push_str("    mov 8(%r13), %r14\n");
        out.push_str("    lea alya_fmt_struct_open(%rip), %rcx\n");
        out.push_str("    mov (%r13), %rdx\n");
        out.push_str("    call printf\n");
        out.push_str("    xor %r15, %r15\n");
        out.push_str(".L_x64_struct_loop:\n");
        out.push_str("    cmp %r14, %r15\n");
        out.push_str("    jge .L_x64_struct_close\n");
        out.push_str("    test %r15, %r15\n");
        out.push_str("    jz .L_x64_struct_print_f\n");
        out.push_str("    lea alya_fmt_struct_comma(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_struct_print_f:\n");
        out.push_str("    lea alya_fmt_struct_field(%rip), %rcx\n");
        out.push_str("    mov 16(%r13, %r15, 8), %rdx\n");
        out.push_str("    mov 8(%r12, %r15, 8), %r8\n");
        out.push_str("    call printf\n");
        out.push_str("    inc %r15\n");
        out.push_str("    jmp .L_x64_struct_loop\n");
        out.push_str(".L_x64_struct_close:\n");
        out.push_str("    lea alya_fmt_struct_close(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_struct_exit:\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jnz .L_x64_struct_not_null\n");
        out.push_str("    lea alya_fmt_struct_null(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    jmp .L_x64_struct_exit\n");
        out.push_str(".L_x64_struct_not_null:\n");
        out.push_str("    mov (%r12), %r13\n");
        out.push_str("    mov 8(%r13), %r14\n");
        out.push_str("    lea alya_fmt_struct_open(%rip), %rdi\n");
        out.push_str("    mov (%r13), %rsi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    xor %r15, %r15\n");
        out.push_str(".L_x64_struct_loop:\n");
        out.push_str("    cmp %r14, %r15\n");
        out.push_str("    jge .L_x64_struct_close\n");
        out.push_str("    test %r15, %r15\n");
        out.push_str("    jz .L_x64_struct_print_f\n");
        out.push_str("    lea alya_fmt_struct_comma(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_struct_print_f:\n");
        out.push_str("    lea alya_fmt_struct_field(%rip), %rdi\n");
        out.push_str("    mov 16(%r13, %r15, 8), %rsi\n");
        out.push_str("    mov 8(%r12, %r15, 8), %rdx\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    inc %r15\n");
        out.push_str("    jmp .L_x64_struct_loop\n");
        out.push_str(".L_x64_struct_close:\n");
        out.push_str("    lea alya_fmt_struct_close(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_struct_exit:\n");
    }
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

}
