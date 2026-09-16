use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_map_hash
    out.push_str(".global alya_map_hash\n");
    out.push_str("alya_map_hash:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rdx\n");
    } else {
        out.push_str("    mov %rdi, %rdx\n");
    }
    out.push_str("    cmp $256, %rdx\n");
    out.push_str("    jb .L_x64_mhash_int\n");
    out.push_str("    mov $5381, %rax\n");
    out.push_str(".L_x64_mhash_loop:\n");
    out.push_str("    movzbq (%rdx), %rcx\n");
    out.push_str("    test %rcx, %rcx\n");
    out.push_str("    jz .L_x64_mhash_done\n");
    out.push_str("    mov %rax, %r8\n");
    out.push_str("    shl $5, %r8\n");
    out.push_str("    add %r8, %rax\n");
    out.push_str("    add %rcx, %rax\n");
    out.push_str("    inc %rdx\n");
    out.push_str("    jmp .L_x64_mhash_loop\n");
    out.push_str(".L_x64_mhash_int:\n");
    out.push_str("    mov %rdx, %rax\n");
    out.push_str(".L_x64_mhash_done:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_map_key_eq / fn_streq
    out.push_str(".global fn_streq\n");
    out.push_str("fn_streq:\n");
    out.push_str(".global alya_map_key_eq\n");
    out.push_str("alya_map_key_eq:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r8\n");
        out.push_str("    mov %rdx, %r9\n");
    } else {
        out.push_str("    mov %rdi, %r8\n");
        out.push_str("    mov %rsi, %r9\n");
    }
    out.push_str("    cmp %r8, %r9\n");
    out.push_str("    je .L_x64_mkeq_true\n");
    out.push_str("    cmp $65536, %r8\n");
    out.push_str("    jb .L_x64_mkeq_false\n");
    out.push_str("    cmp $65536, %r9\n");
    out.push_str("    jb .L_x64_mkeq_false\n");
    out.push_str(".L_x64_mkeq_str:\n");
    out.push_str("    movb (%r8), %al\n");
    out.push_str("    movb (%r9), %cl\n");
    out.push_str("    cmp %al, %cl\n");
    out.push_str("    jne .L_x64_mkeq_false\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x64_mkeq_true\n");
    out.push_str("    inc %r8\n");
    out.push_str("    inc %r9\n");
    out.push_str("    jmp .L_x64_mkeq_str\n");
    out.push_str(".L_x64_mkeq_true:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_mkeq_end\n");
    out.push_str(".L_x64_mkeq_false:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_mkeq_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_strcmp
    out.push_str(".global fn_strcmp\n");
    out.push_str("fn_strcmp:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r8\n");
        out.push_str("    mov %rdx, %r9\n");
    } else {
        out.push_str("    mov %rdi, %r8\n");
        out.push_str("    mov %rsi, %r9\n");
    }
    out.push_str("    cmp %r8, %r9\n");
    out.push_str("    je .L_x64_strcmp_eq\n");
    out.push_str("    cmp $65536, %r8\n");
    out.push_str("    jb .L_x64_strcmp_s1_null\n");
    out.push_str("    cmp $65536, %r9\n");
    out.push_str("    jb .L_x64_strcmp_s2_null\n");
    out.push_str(".L_x64_strcmp_loop:\n");
    out.push_str("    movzbq (%r8), %rax\n");
    out.push_str("    movzbq (%r9), %rcx\n");
    out.push_str("    cmp %al, %cl\n");
    out.push_str("    jne .L_x64_strcmp_diff\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x64_strcmp_eq\n");
    out.push_str("    inc %r8\n");
    out.push_str("    inc %r9\n");
    out.push_str("    jmp .L_x64_strcmp_loop\n");
    out.push_str(".L_x64_strcmp_diff:\n");
    out.push_str("    sub %rcx, %rax\n");
    out.push_str("    jmp .L_x64_strcmp_end\n");
    out.push_str(".L_x64_strcmp_s1_null:\n");
    out.push_str("    cmp $65536, %r9\n");
    out.push_str("    jb .L_x64_strcmp_eq\n");
    out.push_str("    mov $-1, %rax\n");
    out.push_str("    jmp .L_x64_strcmp_end\n");
    out.push_str(".L_x64_strcmp_s2_null:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_strcmp_end\n");
    out.push_str(".L_x64_strcmp_eq:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_strcmp_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_map
    out.push_str(".global fn_map\n");
    out.push_str("fn_map:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    mov $40, %rdx\n");
        out.push_str("    call calloc\n");
        out.push_str("    movq $0x5A110002, (%rax)\n");
        out.push_str("    movq $1, 8(%rax)\n");
        out.push_str("    lea 16(%rax), %rbx\n");
        out.push_str("    movq $0, (%rbx)\n");
        out.push_str("    movq $64, 8(%rbx)\n");
        out.push_str("    mov $64, %rcx\n");
        out.push_str("    mov $24, %rdx\n");
        out.push_str("    call calloc\n");
        out.push_str("    mov %rax, 16(%rbx)\n");
        out.push_str("    mov %rbx, %rax\n");
        out.push_str("    add $40, %rsp\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov $1, %rdi\n");
        out.push_str("    mov $40, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
        out.push_str("    movq $0x5A110002, (%rax)\n");
        out.push_str("    movq $1, 8(%rax)\n");
        out.push_str("    lea 16(%rax), %rbx\n");
        out.push_str("    movq $0, (%rbx)\n");
        out.push_str("    movq $64, 8(%rbx)\n");
        out.push_str("    mov $64, %rdi\n");
        out.push_str("    mov $24, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
        out.push_str("    mov %rax, 16(%rbx)\n");
        out.push_str("    mov %rbx, %rax\n");
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    addq $1576, alya_allocated_bytes(%rip)\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_set
    out.push_str(".global fn_set\n");
    out.push_str("fn_set:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $72, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    mov %r8, %r14\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
        out.push_str("    mov %rdx, %r14\n");
    }
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_set_done\n");
    out.push_str("    mov (%r12), %rax\n");
    out.push_str("    shl $1, %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jl .L_x64_set_no_resize\n");
    out.push_str("    mov 8(%r12), %rax\n");
    out.push_str("    shl $1, %rax\n");
    out.push_str("    mov %rax, 48(%rsp)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov $24, %rdx\n");
        out.push_str("    call calloc\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov $24, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    mov %rax, 40(%rsp)\n");
    out.push_str("    mov 48(%rsp), %r11\n");
    out.push_str("    imul $24, %r11, %r11\n");
    out.push_str("    add %r11, alya_allocated_bytes(%rip)\n");
    out.push_str("    mov 48(%rsp), %rax\n");
    out.push_str("    dec %rax\n");
    out.push_str("    mov %rax, 56(%rsp)\n");
    out.push_str("    movq $0, 64(%rsp)\n");
    out.push_str(".L_x64_rehash_loop:\n");
    out.push_str("    mov 64(%rsp), %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jge .L_x64_rehash_done\n");
    out.push_str("    mov 16(%r12), %rcx\n");
    out.push_str("    lea (%rax, %rax, 2), %rbx\n");
    out.push_str("    shl $3, %rbx\n");
    out.push_str("    add %rcx, %rbx\n");
    out.push_str("    cmpq $1, 16(%rbx)\n");
    out.push_str("    jne .L_x64_rehash_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov (%rbx), %rcx\n");
        out.push_str("    call alya_map_hash\n");
    } else {
        out.push_str("    mov (%rbx), %rdi\n");
        out.push_str("    call alya_map_hash\n");
    }
    out.push_str("    and 56(%rsp), %rax\n");
    out.push_str("    mov 40(%rsp), %rdi\n");
    out.push_str(".L_x64_rehash_probe:\n");
    out.push_str("    lea (%rax, %rax, 2), %r8\n");
    out.push_str("    shl $3, %r8\n");
    out.push_str("    add %rdi, %r8\n");
    out.push_str("    cmpq $0, 16(%r8)\n");
    out.push_str("    je .L_x64_rehash_put\n");
    out.push_str("    inc %rax\n");
    out.push_str("    and 56(%rsp), %rax\n");
    out.push_str("    jmp .L_x64_rehash_probe\n");
    out.push_str(".L_x64_rehash_put:\n");
    out.push_str("    mov 16(%r12), %rcx\n");
    out.push_str("    mov 64(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %rbx\n");
    out.push_str("    shl $3, %rbx\n");
    out.push_str("    add %rcx, %rbx\n");
    out.push_str("    mov (%rbx), %r9\n");
    out.push_str("    mov %r9, (%r8)\n");
    out.push_str("    mov 8(%rbx), %r9\n");
    out.push_str("    mov %r9, 8(%r8)\n");
    out.push_str("    movq $1, 16(%r8)\n");
    out.push_str(".L_x64_rehash_next:\n");
    out.push_str("    incq 64(%rsp)\n");
    out.push_str("    jmp .L_x64_rehash_loop\n");
    out.push_str(".L_x64_rehash_done:\n");
    out.push_str("    mov 48(%rsp), %rax\n");
    out.push_str("    mov %rax, 8(%r12)\n");
    out.push_str("    mov 40(%rsp), %rax\n");
    out.push_str("    mov %rax, 16(%r12)\n");
    out.push_str(".L_x64_set_no_resize:\n");
    out.push_str("    mov 8(%r12), %rax\n");
    out.push_str("    dec %rax\n");
    out.push_str("    mov %rax, 48(%rsp)\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_map_hash\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_map_hash\n");
    }
    out.push_str("    and 48(%rsp), %rax\n");
    out.push_str("    mov %rax, 40(%rsp)\n");
    out.push_str("    movq $-1, 32(%rsp)\n");
    out.push_str("    movq $0, 56(%rsp)\n");
    out.push_str(".L_x64_set_probe:\n");
    out.push_str("    mov 56(%rsp), %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jge .L_x64_set_use_tomb\n");
    out.push_str("    mov 40(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r15\n");
    out.push_str("    shl $3, %r15\n");
    out.push_str("    add 16(%r12), %r15\n");
    out.push_str("    mov 16(%r15), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_set_empty\n");
    out.push_str("    cmp $2, %rax\n");
    out.push_str("    jne .L_x64_set_check_key\n");
    out.push_str("    cmpq $-1, 32(%rsp)\n");
    out.push_str("    jne .L_x64_set_next\n");
    out.push_str("    mov 40(%rsp), %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    jmp .L_x64_set_next\n");
    out.push_str(".L_x64_set_check_key:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov (%r15), %rcx\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    call alya_map_key_eq\n");
    } else {
        out.push_str("    mov (%r15), %rdi\n");
        out.push_str("    mov %r13, %rsi\n");
        out.push_str("    call alya_map_key_eq\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_set_next\n");
    out.push_str("    mov 40(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r15\n");
    out.push_str("    shl $3, %r15\n");
    out.push_str("    add 16(%r12), %r15\n");
    out.push_str("    mov %r14, 8(%r15)\n");
    out.push_str("    jmp .L_x64_set_done\n");
    out.push_str(".L_x64_set_next:\n");
    out.push_str("    incq 56(%rsp)\n");
    out.push_str("    mov 40(%rsp), %rax\n");
    out.push_str("    inc %rax\n");
    out.push_str("    and 48(%rsp), %rax\n");
    out.push_str("    mov %rax, 40(%rsp)\n");
    out.push_str("    jmp .L_x64_set_probe\n");
    out.push_str(".L_x64_set_empty:\n");
    out.push_str("    cmpq $-1, 32(%rsp)\n");
    out.push_str("    je .L_x64_set_insert_here\n");
    out.push_str(".L_x64_set_use_tomb:\n");
    out.push_str("    cmpq $-1, 32(%rsp)\n");
    out.push_str("    je .L_x64_set_done\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r15\n");
    out.push_str("    shl $3, %r15\n");
    out.push_str("    add 16(%r12), %r15\n");
    out.push_str(".L_x64_set_insert_here:\n");
    out.push_str("    mov %r13, (%r15)\n");
    out.push_str("    mov %r14, 8(%r15)\n");
    out.push_str("    movq $1, 16(%r15)\n");
    out.push_str("    incq (%r12)\n");
    out.push_str(".L_x64_set_done:\n");
    out.push_str("    add $72, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_get
    out.push_str(".global fn_get\n");
    out.push_str("fn_get:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_get_ret\n");
    out.push_str("    cmp $65536, %r12\n");
    out.push_str("    jb .L_x64_get_ret\n");
    out.push_str("    movq -16(%r12), %rax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    je .L_x64_get_array\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_map_hash\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_map_hash\n");
    }
    out.push_str("    mov 8(%r12), %rdx\n");
    out.push_str("    dec %rdx\n");
    out.push_str("    and %rdx, %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    mov %rdx, 40(%rsp)\n");
    out.push_str("    movq $0, 48(%rsp)\n");
    out.push_str(".L_x64_get_loop:\n");
    out.push_str("    mov 48(%rsp), %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jge .L_x64_get_not_found\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r14\n");
    out.push_str("    shl $3, %r14\n");
    out.push_str("    add 16(%r12), %r14\n");
    out.push_str("    cmpq $0, 16(%r14)\n");
    out.push_str("    je .L_x64_get_not_found\n");
    out.push_str("    cmpq $1, 16(%r14)\n");
    out.push_str("    jne .L_x64_get_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov (%r14), %rcx\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    call alya_map_key_eq\n");
    } else {
        out.push_str("    mov (%r14), %rdi\n");
        out.push_str("    mov %r13, %rsi\n");
        out.push_str("    call alya_map_key_eq\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_get_found\n");
    out.push_str(".L_x64_get_next:\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    inc %rax\n");
    out.push_str("    and 40(%rsp), %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    incq 48(%rsp)\n");
    out.push_str("    jmp .L_x64_get_loop\n");
    out.push_str(".L_x64_get_found:\n");
    out.push_str("    mov 8(%r14), %rax\n");
    out.push_str("    jmp .L_x64_get_ret\n");
    out.push_str(".L_x64_get_array:\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    js .L_x64_get_not_found\n");
    out.push_str("    cmpq (%r12), %r13\n");
    out.push_str("    jae .L_x64_get_not_found\n");
    out.push_str("    movq 16(%r12), %rax\n");
    out.push_str("    movq (%rax, %r13, 8), %rax\n");
    out.push_str("    jmp .L_x64_get_ret\n");
    out.push_str(".L_x64_get_not_found:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_get_ret:\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_has
    out.push_str(".global fn_has\n");
    out.push_str("fn_has:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_has_ret\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_map_hash\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_map_hash\n");
    }
    out.push_str("    mov 8(%r12), %rdx\n");
    out.push_str("    dec %rdx\n");
    out.push_str("    and %rdx, %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    mov %rdx, 40(%rsp)\n");
    out.push_str("    movq $0, 48(%rsp)\n");
    out.push_str(".L_x64_has_loop:\n");
    out.push_str("    mov 48(%rsp), %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jge .L_x64_has_not_found\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r14\n");
    out.push_str("    shl $3, %r14\n");
    out.push_str("    add 16(%r12), %r14\n");
    out.push_str("    cmpq $0, 16(%r14)\n");
    out.push_str("    je .L_x64_has_not_found\n");
    out.push_str("    cmpq $1, 16(%r14)\n");
    out.push_str("    jne .L_x64_has_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov (%r14), %rcx\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    call alya_map_key_eq\n");
    } else {
        out.push_str("    mov (%r14), %rdi\n");
        out.push_str("    mov %r13, %rsi\n");
        out.push_str("    call alya_map_key_eq\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_has_found\n");
    out.push_str(".L_x64_has_next:\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    inc %rax\n");
    out.push_str("    and 40(%rsp), %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    incq 48(%rsp)\n");
    out.push_str("    jmp .L_x64_has_loop\n");
    out.push_str(".L_x64_has_found:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_has_ret\n");
    out.push_str(".L_x64_has_not_found:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_has_ret:\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_remove
    out.push_str(".global fn_remove\n");
    out.push_str("fn_remove:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    mov %rsi, %r13\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_rem_ret\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    call alya_map_hash\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    call alya_map_hash\n");
    }
    out.push_str("    mov 8(%r12), %rdx\n");
    out.push_str("    dec %rdx\n");
    out.push_str("    and %rdx, %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    mov %rdx, 40(%rsp)\n");
    out.push_str("    movq $0, 48(%rsp)\n");
    out.push_str(".L_x64_rem_loop:\n");
    out.push_str("    mov 48(%rsp), %rax\n");
    out.push_str("    cmp 8(%r12), %rax\n");
    out.push_str("    jge .L_x64_rem_not_found\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    lea (%rax, %rax, 2), %r14\n");
    out.push_str("    shl $3, %r14\n");
    out.push_str("    add 16(%r12), %r14\n");
    out.push_str("    cmpq $0, 16(%r14)\n");
    out.push_str("    je .L_x64_rem_not_found\n");
    out.push_str("    cmpq $1, 16(%r14)\n");
    out.push_str("    jne .L_x64_rem_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov (%r14), %rcx\n");
        out.push_str("    mov %r13, %rdx\n");
        out.push_str("    call alya_map_key_eq\n");
    } else {
        out.push_str("    mov (%r14), %rdi\n");
        out.push_str("    mov %r13, %rsi\n");
        out.push_str("    call alya_map_key_eq\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_rem_found\n");
    out.push_str(".L_x64_rem_next:\n");
    out.push_str("    mov 32(%rsp), %rax\n");
    out.push_str("    inc %rax\n");
    out.push_str("    and 40(%rsp), %rax\n");
    out.push_str("    mov %rax, 32(%rsp)\n");
    out.push_str("    incq 48(%rsp)\n");
    out.push_str("    jmp .L_x64_rem_loop\n");
    out.push_str(".L_x64_rem_found:\n");
    out.push_str("    movq $2, 16(%r14)\n");
    out.push_str("    decq (%r12)\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_rem_ret\n");
    out.push_str(".L_x64_rem_not_found:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_rem_ret:\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_keys
    out.push_str(".global fn_keys\n");
    out.push_str("fn_keys:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
    }
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_array_new\n");
    } else {
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    mov %rax, %r13\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_keys_done\n");
    out.push_str("    mov 16(%r12), %r14\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_keys_loop:\n");
    out.push_str("    cmp 8(%r12), %rbx\n");
    out.push_str("    jge .L_x64_keys_done\n");
    out.push_str("    lea (%rbx, %rbx, 2), %r15\n");
    out.push_str("    shl $3, %r15\n");
    out.push_str("    add %r14, %r15\n");
    out.push_str("    cmpq $1, 16(%r15)\n");
    out.push_str("    jne .L_x64_keys_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    mov (%r15), %rdx\n");
        out.push_str("    call alya_array_push\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    mov (%r15), %rsi\n");
        out.push_str("    call alya_array_push\n");
    }
    out.push_str(".L_x64_keys_next:\n");
    out.push_str("    inc %rbx\n");
    out.push_str("    jmp .L_x64_keys_loop\n");
    out.push_str(".L_x64_keys_done:\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_values
    out.push_str(".global fn_values\n");
    out.push_str("fn_values:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    sub $56, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r12\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_array_new\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str("    call alya_array_new\n");
    }
    out.push_str("    mov %rax, %r13\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_vals_done\n");
    out.push_str("    mov 16(%r12), %r14\n");
    out.push_str("    xor %rbx, %rbx\n");
    out.push_str(".L_x64_vals_loop:\n");
    out.push_str("    cmp 8(%r12), %rbx\n");
    out.push_str("    jge .L_x64_vals_done\n");
    out.push_str("    lea (%rbx, %rbx, 2), %r15\n");
    out.push_str("    shl $3, %r15\n");
    out.push_str("    add %r14, %r15\n");
    out.push_str("    cmpq $1, 16(%r15)\n");
    out.push_str("    jne .L_x64_vals_next\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %r13, %rcx\n");
        out.push_str("    mov 8(%r15), %rdx\n");
        out.push_str("    call alya_array_push\n");
    } else {
        out.push_str("    mov %r13, %rdi\n");
        out.push_str("    mov 8(%r15), %rsi\n");
        out.push_str("    call alya_array_push\n");
    }
    out.push_str(".L_x64_vals_next:\n");
    out.push_str("    inc %rbx\n");
    out.push_str("    jmp .L_x64_vals_loop\n");
    out.push_str(".L_x64_vals_done:\n");
    out.push_str("    mov %r13, %rax\n");
    out.push_str("    add $56, %rsp\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_print_map
    out.push_str(".global alya_print_map\n");
    out.push_str("alya_print_map:\n");
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
        out.push_str("    jnz .L_x64_pmap_not_null\n");
        out.push_str("    lea alya_fmt_map_null(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    jmp .L_x64_pmap_exit\n");
        out.push_str(".L_x64_pmap_not_null:\n");
        out.push_str("    cmpq $0, (%r12)\n");
        out.push_str("    jne .L_x64_pmap_has_items\n");
        out.push_str("    lea alya_fmt_map_empty(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    jmp .L_x64_pmap_exit\n");
        out.push_str(".L_x64_pmap_has_items:\n");
        out.push_str("    lea alya_fmt_map_open(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    xor %rbx, %rbx\n");
        out.push_str("    mov 16(%r12), %r13\n");
        out.push_str("    xor %r14, %r14\n");
        out.push_str(".L_x64_pmap_loop:\n");
        out.push_str("    cmp 8(%r12), %r14\n");
        out.push_str("    jge .L_x64_pmap_close\n");
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    cmpq $1, 16(%rax)\n");
        out.push_str("    jne .L_x64_pmap_next\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_pmap_print_pair\n");
        out.push_str("    lea alya_fmt_arr_comma(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_pmap_print_pair:\n");
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    mov (%rax), %rdx\n");
        out.push_str("    cmp $256, %rdx\n");
        out.push_str("    jb .L_x64_pmap_key_num\n");
        out.push_str("    lea alya_fmt_prompt(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    jmp .L_x64_pmap_colon\n");
        out.push_str(".L_x64_pmap_key_num:\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_pmap_colon:\n");
        out.push_str("    lea alya_fmt_map_colon(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    mov 8(%rax), %rdx\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    inc %rbx\n");
        out.push_str(".L_x64_pmap_next:\n");
        out.push_str("    inc %r14\n");
        out.push_str("    jmp .L_x64_pmap_loop\n");
        out.push_str(".L_x64_pmap_close:\n");
        out.push_str("    lea alya_fmt_map_close(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str(".L_x64_pmap_exit:\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %r12\n");
        out.push_str("    test %r12, %r12\n");
        out.push_str("    jnz .L_x64_pmap_not_null\n");
        out.push_str("    lea alya_fmt_map_null(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    jmp .L_x64_pmap_exit\n");
        out.push_str(".L_x64_pmap_not_null:\n");
        out.push_str("    cmpq $0, (%r12)\n");
        out.push_str("    jne .L_x64_pmap_has_items\n");
        out.push_str("    lea alya_fmt_map_empty(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    jmp .L_x64_pmap_exit\n");
        out.push_str(".L_x64_pmap_has_items:\n");
        out.push_str("    lea alya_fmt_map_open(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    xor %rbx, %rbx\n");
        out.push_str("    mov 16(%r12), %r13\n");
        out.push_str("    xor %r14, %r14\n");
        out.push_str(".L_x64_pmap_loop:\n");
        out.push_str("    cmp 8(%r12), %r14\n");
        out.push_str("    jge .L_x64_pmap_close\n");
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    cmpq $1, 16(%rax)\n");
        out.push_str("    jne .L_x64_pmap_next\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_pmap_print_pair\n");
        out.push_str("    lea alya_fmt_arr_comma(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_pmap_print_pair:\n");
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    mov (%rax), %rsi\n");
        out.push_str("    cmp $256, %rsi\n");
        out.push_str("    jb .L_x64_pmap_key_num\n");
        out.push_str("    lea alya_fmt_prompt(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    jmp .L_x64_pmap_colon\n");
        out.push_str(".L_x64_pmap_key_num:\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_pmap_colon:\n");
        out.push_str("    lea alya_fmt_map_colon(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    lea (%r14, %r14, 2), %rax\n");
        out.push_str("    shl $3, %rax\n");
        out.push_str("    add %r13, %rax\n");
        out.push_str("    mov 8(%rax), %rsi\n");
        out.push_str("    lea alya_fmt_arr_elem(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    inc %rbx\n");
        out.push_str(".L_x64_pmap_next:\n");
        out.push_str("    inc %r14\n");
        out.push_str("    jmp .L_x64_pmap_loop\n");
        out.push_str(".L_x64_pmap_close:\n");
        out.push_str("    lea alya_fmt_map_close(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str(".L_x64_pmap_exit:\n");
    }
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

}
