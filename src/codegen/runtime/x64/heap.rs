use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_alloc
    out.push_str(".global fn_alloc\n");
    out.push_str("fn_alloc:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    add %rcx, alya_allocated_bytes(%rip)\n");
        out.push_str("    call malloc\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_alloc_done\n");
        out.push_str("    push %rax\n");
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov %rbx, %rdx\n");
        out.push_str("    mov $5, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_alloc\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    pop %rax\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    add %rdi, alya_allocated_bytes(%rip)\n");
        out.push_str(&format!("    call {}malloc\n", p));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_alloc_done\n");
        out.push_str("    push %rax\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str("    mov $5, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_mem_track_alloc\n");
        out.push_str("    pop %rax\n");
    }
    out.push_str(".L_x64_alloc_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_free
    out.push_str(".global fn_free\n");
    out.push_str("fn_free:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    test %rcx, %rcx\n");
        out.push_str("    jz .L_x64_free_done\n");
        out.push_str("    push %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    pop %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
        out.push_str("    jz .L_x64_free_done\n");
        out.push_str("    push %rdi\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    pop %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str(".L_x64_free_done:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_realloc
    out.push_str(".global fn_realloc\n");
    out.push_str("fn_realloc:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    add %rdx, alya_allocated_bytes(%rip)\n");
        out.push_str("    call realloc\n");
    } else {
        out.push_str("    add %rsi, alya_allocated_bytes(%rip)\n");
        out.push_str(&format!("    call {}realloc\n", p));
    }
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_copy_mem
    out.push_str(".global fn_copy_mem\n");
    out.push_str("fn_copy_mem:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    call memcpy\n");
    } else {
        out.push_str(&format!("    call {}memcpy\n", p));
    }
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_zero_mem
    out.push_str(".global fn_zero_mem\n");
    out.push_str("fn_zero_mem:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    mov %rdx, %r8\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    call memset\n");
    } else {
        out.push_str("    mov %rsi, %rdx\n");
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str(&format!("    call {}memset\n", p));
    }
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_peek_byte
    out.push_str(".global fn_peek_byte\n");
    out.push_str("fn_peek_byte:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movzbq (%rcx), %rax\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movzbq (%rdi), %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_poke_byte
    out.push_str(".global fn_poke_byte\n");
    out.push_str("fn_poke_byte:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movb %r8b, (%rcx)\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movb %dl, (%rdi)\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_peek_int
    out.push_str(".global fn_peek_int\n");
    out.push_str("fn_peek_int:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movq (%rcx), %rax\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movq (%rdi), %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_poke_int
    out.push_str(".global fn_poke_int\n");
    out.push_str("fn_poke_int:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movq %r8, (%rcx)\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movq %rdx, (%rdi)\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_from_ptr
    out.push_str(".global fn_str_from_ptr\n");
    out.push_str("fn_str_from_ptr:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    test %rcx, %rcx\n");
        out.push_str("    jnz .L_x64_sfp_ok\n");
        out.push_str("    lea alya_str_empty(%rip), %rax\n");
        out.push_str("    jmp .L_x64_sfp_ret\n");
        out.push_str(".L_x64_sfp_ok:\n");
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
        out.push_str("    jnz .L_x64_sfp_ok\n");
        out.push_str("    lea alya_str_empty(%rip), %rax\n");
        out.push_str("    jmp .L_x64_sfp_ret\n");
        out.push_str(".L_x64_sfp_ok:\n");
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str(".L_x64_sfp_ret:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_to_ptr
    out.push_str(".global fn_str_to_ptr\n");
    out.push_str("fn_str_to_ptr:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_mem_allocated
    out.push_str(".global fn_mem_allocated\n");
    out.push_str("fn_mem_allocated:\n");
    out.push_str("    mov alya_allocated_bytes(%rip), %rax\n");
    out.push_str("    ret\n\n");

    // fn_mem_reset_alloc
    out.push_str(".global fn_mem_reset_alloc\n");
    out.push_str("fn_mem_reset_alloc:\n");
    out.push_str("    movq $0, alya_allocated_bytes(%rip)\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    ret\n\n");

    // fn_str_clone
    out.push_str(".global fn_str_clone\n");
    out.push_str("fn_str_clone:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_sclone_empty\n");
    out.push_str("    mov %rbx, %r12\n");
    out.push_str("    xor %r13, %r13\n");
    out.push_str(".L_x64_sclone_len:\n");
    out.push_str("    cmpb $0, (%r12)\n");
    out.push_str("    je .L_x64_sclone_alloc\n");
    out.push_str("    inc %r12\n");
    out.push_str("    inc %r13\n");
    out.push_str("    jmp .L_x64_sclone_len\n");
    out.push_str(".L_x64_sclone_alloc:\n");
    out.push_str("    lea 1(%r13), %r12\n");
    if is_win {
        out.push_str("    mov %r12, %rcx\n");
        out.push_str("    call malloc\n");
    } else {
        out.push_str("    mov %r12, %rdi\n");
        out.push_str(&format!("    call {}malloc\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_sclone_empty\n");
    out.push_str("    add %r12, alya_allocated_bytes(%rip)\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov %rbx, %rdx\n");
        out.push_str("    mov %r12, %r8\n");
        out.push_str("    call memcpy\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str("    mov %r12, %rdx\n");
        out.push_str(&format!("    call {}memcpy\n", p));
    }
    out.push_str("    push %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    mov %r12, %rdx\n");
        out.push_str("    mov $4, %r8\n");
        out.push_str("    xor %r9, %r9\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_alloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    mov %r12, %rsi\n");
        out.push_str("    mov $4, %rdx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call alya_mem_track_alloc\n");
    }
    out.push_str("    pop %rax\n");
    out.push_str("    jmp .L_x64_sclone_done\n");
    out.push_str(".L_x64_sclone_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_sclone_done:\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_str_free
    out.push_str(".global fn_str_free\n");
    out.push_str("fn_str_free:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    test %rcx, %rcx\n");
        out.push_str("    jz .L_x64_sfree_done\n");
        out.push_str("    push %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    pop %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
        out.push_str("    jz .L_x64_sfree_done\n");
        out.push_str("    push %rdi\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    pop %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str(".L_x64_sfree_done:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_rc_retain
    out.push_str(".global fn_rc_retain\n");
    out.push_str("fn_rc_retain:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    mov %rcx, %r11\n");
    } else {
        out.push_str("    mov %rdi, %r11\n");
    }
    out.push_str("    test $7, %r11\n");
    out.push_str("    jnz .L_x64_rc_retain_done\n");
    out.push_str("    cmp $65536, %r11\n");
    out.push_str("    jbe .L_x64_rc_retain_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    ja .L_x64_rc_retain_done\n");
    out.push_str("    movl -16(%r11), %eax\n");
    out.push_str("    cmp $0x5A110001, %rax\n");
    out.push_str("    je .L_x64_rc_retain_ok\n");
    out.push_str("    cmp $0x5A110002, %rax\n");
    out.push_str("    je .L_x64_rc_retain_ok\n");
    out.push_str("    cmp $0x5A110003, %rax\n");
    out.push_str("    je .L_x64_rc_retain_ok\n");
    out.push_str("    cmp $0x5A110004, %rax\n");
    out.push_str("    jne .L_x64_rc_retain_done\n");
    out.push_str(".L_x64_rc_retain_ok:\n");
    out.push_str("    lock incq -8(%r11)\n");
    out.push_str("    movl $0, -12(%r11)\n");
    out.push_str(".L_x64_rc_retain_done:\n");
    out.push_str("    mov %r11, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_rc_release
    out.push_str(".global fn_rc_release\n");
    out.push_str("fn_rc_release:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test $7, %rbx\n");
    out.push_str("    jnz .L_x64_rc_rel_done\n");
    out.push_str("    cmp $65536, %rbx\n");
    out.push_str("    jbe .L_x64_rc_rel_done\n");
    out.push_str("    mov $0x00007fffffffffff, %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    ja .L_x64_rc_rel_done\n");
    out.push_str("    movl -16(%rbx), %r12d\n");
    out.push_str("    cmp $0x5A110001, %r12\n");
    out.push_str("    je .L_x64_rc_rel_ok\n");
    out.push_str("    cmp $0x5A110002, %r12\n");
    out.push_str("    je .L_x64_rc_rel_ok\n");
    out.push_str("    cmp $0x5A110003, %r12\n");
    out.push_str("    je .L_x64_rc_rel_ok\n");
    out.push_str("    cmp $0x5A110004, %r12\n");
    out.push_str("    jne .L_x64_rc_rel_done\n");
    out.push_str(".L_x64_rc_rel_ok:\n");
    out.push_str("    lock decq -8(%rbx)\n");
    out.push_str("    jnz .L_x64_rc_rel_purple\n");
    out.push_str("    movl $0, -12(%rbx)\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_mem_track_free\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    call alya_mem_track_free\n");
    }
    out.push_str("    cmp $0x5A110001, %r12\n");
    out.push_str("    je .L_x64_rc_free_inner\n");
    out.push_str("    cmp $0x5A110002, %r12\n");
    out.push_str("    je .L_x64_rc_free_inner\n");
    out.push_str("    jmp .L_x64_rc_free_outer\n");
    out.push_str(".L_x64_rc_free_inner:\n");
    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_rc_free_outer\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str(".L_x64_rc_free_outer:\n");
    out.push_str("    lea -16(%rbx), %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    jmp .L_x64_rc_rel_done\n");
    out.push_str(".L_x64_rc_rel_purple:\n");
    out.push_str("    cmp $0x5A110004, %r12\n");
    out.push_str("    je .L_x64_rc_rel_done\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_gc_add_purple\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    call fn_gc_add_purple\n");
    }
    out.push_str(".L_x64_rc_rel_done:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $48, %rsp\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_rc_count
    out.push_str(".global fn_rc_count\n");
    out.push_str("fn_rc_count:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    test $7, %rax\n");
    out.push_str("    jnz .L_x64_rcc_zero\n");
    out.push_str("    cmp $65536, %rax\n");
    out.push_str("    jbe .L_x64_rcc_zero\n");
    out.push_str("    mov $0x00007fffffffffff, %rdx\n");
    out.push_str("    cmp %rdx, %rax\n");
    out.push_str("    ja .L_x64_rcc_zero\n");
    out.push_str("    movl -16(%rax), %edx\n");
    out.push_str("    cmp $0x5A110001, %rdx\n");
    out.push_str("    je .L_x64_rcc_ok\n");
    out.push_str("    cmp $0x5A110002, %rdx\n");
    out.push_str("    je .L_x64_rcc_ok\n");
    out.push_str("    cmp $0x5A110003, %rdx\n");
    out.push_str("    je .L_x64_rcc_ok\n");
    out.push_str("    cmp $0x5A110004, %rdx\n");
    out.push_str("    je .L_x64_rcc_ok\n");
    out.push_str(".L_x64_rcc_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_rcc_ok:\n");
    out.push_str("    movq -8(%rax), %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
