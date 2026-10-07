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

    // fn_mem_peek_f32(ptr, offset) -> float
    // Loads a 32-bit float and widens it to f64 (exact widening).
    // Alya floats travel as raw f64 bits in value slots.
    out.push_str(".global fn_mem_peek_f32\n");
    out.push_str("fn_mem_peek_f32:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movss (%rcx), %xmm0\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movss (%rdi), %xmm0\n");
    }
    out.push_str("    cvtss2sd %xmm0, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_f32(ptr, offset, val) -> void
    // Narrows an f64 value-slot float to 32-bit storage (hardware
    // round-to-nearest-even via cvtsd2ss).
    out.push_str(".global fn_mem_poke_f32\n");
    out.push_str("fn_mem_poke_f32:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    movq %r8, %xmm0\n");
        out.push_str("    cvtsd2ss %xmm0, %xmm0\n");
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movss %xmm0, (%rcx)\n");
    } else {
        out.push_str("    movq %rdx, %xmm0\n");
        out.push_str("    cvtsd2ss %xmm0, %xmm0\n");
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movss %xmm0, (%rdi)\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_mem_peek_i32(ptr, offset) -> int
    // Loads a 32-bit int with sign extension into the 64-bit slot.
    out.push_str(".global fn_mem_peek_i32\n");
    out.push_str("fn_mem_peek_i32:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movslq (%rcx), %rax\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movslq (%rdi), %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_i32(ptr, offset, val) -> void
    // Stores the low 32 bits of the int value slot.
    out.push_str(".global fn_mem_poke_i32\n");
    out.push_str("fn_mem_poke_i32:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    add %rdx, %rcx\n");
        out.push_str("    movl %r8d, (%rcx)\n");
    } else {
        out.push_str("    add %rsi, %rdi\n");
        out.push_str("    movl %edx, (%rdi)\n");
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

    // alya_mem_header_readable(value): 1 if the 16 header bytes at
    // value-16 are fully committed+readable, else 0. Never faults.
    // Windows-only (alya-lang/alya#117): IsBadReadPtr cannot be used
    // here because its internal probe fault is stolen by our own VEH
    // crash handler, killing the process instead of returning nonzero.
    // VirtualQuery is a pure query with no fault path. Callers pass
    // the value; the header span is derived inside.
    if is_win {
        out.push_str(".global alya_mem_header_readable\n");
        out.push_str("alya_mem_header_readable:\n");
        out.push_str("    push %rbp\n");
        out.push_str("    mov %rsp, %rbp\n");
        // 96 bytes: 32 shadow + 48 struct + 16 save pad.
        out.push_str("    sub $96, %rsp\n");
        out.push_str("    mov %rcx, 80(%rsp)\n");
        out.push_str("    lea -16(%rcx), %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    mov $48, %r8\n");
        out.push_str("    call VirtualQuery\n");
        out.push_str("    cmp $48, %rax\n");
        out.push_str("    jne .L_x64_mhr_no\n");
        // State == MEM_COMMIT (0x1000)?
        out.push_str("    cmpl $0x1000, 64(%rsp)\n");
        out.push_str("    jne .L_x64_mhr_no\n");
        // Base <= hdr?
        out.push_str("    mov 32(%rsp), %rax\n");
        out.push_str("    mov 80(%rsp), %rcx\n");
        out.push_str("    sub $16, %rcx\n");
        out.push_str("    cmp %rax, %rcx\n");
        out.push_str("    jb .L_x64_mhr_no\n");
        // hdr+16 <= Base+Size?
        out.push_str("    mov 56(%rsp), %rax\n");
        out.push_str("    add 32(%rsp), %rax\n");
        out.push_str("    mov 80(%rsp), %rcx\n");
        out.push_str("    cmp %rax, %rcx\n");
        out.push_str("    ja .L_x64_mhr_no\n");
        out.push_str("    mov $1, %eax\n");
        out.push_str("    jmp .L_x64_mhr_done\n");
        out.push_str(".L_x64_mhr_no:\n");
        out.push_str("    xor %eax, %eax\n");
        out.push_str(".L_x64_mhr_done:\n");
        out.push_str("    add $96, %rsp\n");
        out.push_str("    mov %rbp, %rsp\n");
        out.push_str("    pop %rbp\n");
        out.push_str("    ret\n\n");
    }

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
    out.push_str("    lea alya_rodata_start(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_ret_chk_str_buf\n");
    out.push_str("    lea alya_rodata_end(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    out.push_str(".L_x64_rc_ret_chk_str_buf:\n");
    out.push_str("    lea alya_str_buf(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_ret_chk_tag\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    // Stable strings are immortal and never refcounted: skipping the
    // header read also avoids gambling on whatever bytes precede the
    // region (a magic match would corrupt refcounts).
    out.push_str("    lea alya_str_stable(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_ret_chk_tag\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    out.push_str(".L_x64_rc_ret_chk_tag:\n");
    // Readability probe (alya-lang/alya#117): raw big ints in unmapped
    // gaps (e.g. 1700000000 on Linux) reach this stage and fault on the
    // header read below. Verify the 16 header bytes are mapped first;
    // unmapped -> skip (safe direction: at worst a leak, never a
    // fault). Fires only for real heap objects and high-gap ints;
    // small ints and immortal regions return earlier (no syscall).
    if is_win {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        out.push_str("    push %r11\n");
        out.push_str("    mov %r11, %rcx\n");
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    call alya_mem_header_readable\n");
        out.push_str("    add $40, %rsp\n");
        out.push_str("    pop %r11\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    jz .L_x64_rc_retain_done\n");
    } else {
        // msync returns 0 when mapped, ENOMEM otherwise. Aligned base
        // + exact span avoids over-probing into neighbors. Pad to keep
        // the call 16-aligned after the push.
        out.push_str("    push %r11\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    lea -16(%r11), %rax\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    and $-4096, %rdi\n");
        out.push_str("    mov %r11, %rsi\n");
        out.push_str("    sub %rdi, %rsi\n");
        out.push_str("    mov $1, %rdx\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    call _msync\n");
        } else {
            out.push_str("    call msync\n");
        }
        out.push_str("    add $8, %rsp\n");
        // Save the result before restoring: pop would overwrite rax
        // and the test would read the pointer (always nonzero->skip).
        out.push_str("    mov %eax, %r10d\n");
        out.push_str("    pop %r11\n");
        out.push_str("    test %r10d, %r10d\n");
        out.push_str("    jnz .L_x64_rc_retain_done\n");
    }
    // Shared tag-read entry: fn_rc_retain_direct jumps here after its
    // own (probe-free) guards. Same registers (r11 value), same frame
    // shape, so the shared tail below serves both.
    out.push_str(".L_x64_rc_ret_tag_read:\n");
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

    // fn_rc_retain_direct(value): same contract as fn_rc_retain but
    // WITHOUT the readability probe. Call ONLY with statically-proven
    // heap values (`is_heap_expression`): literals, typed
    // array/map/struct locals, proven calls. Their headers are our own
    // live allocations, always mapped, so the syscall probe
    // (msync/VirtualQuery, ~100ns+ per op) is pure overhead — it cost up
    // to 6x on allocation-heavy benchmarks. Dynamic or unknown values
    // MUST use the probed fn_rc_retain (alya-lang/alya#117: raw big ints
    // fault on the header read). Guards run first, then control joins
    // the shared tag-read tail (same registers and frame shape).
    out.push_str(".global fn_rc_retain_direct\n");
    out.push_str("fn_rc_retain_direct:\n");
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
    out.push_str("    lea alya_rodata_start(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_direct_chk_str_buf\n");
    out.push_str("    lea alya_rodata_end(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    out.push_str(".L_x64_rc_retain_direct_chk_str_buf:\n");
    out.push_str("    lea alya_str_buf(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_ret_tag_read\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    out.push_str("    lea alya_str_stable(%rip), %rax\n");
    out.push_str("    cmp %rax, %r11\n");
    out.push_str("    jb .L_x64_rc_ret_tag_read\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %r11\n");
    out.push_str("    jb .L_x64_rc_retain_done\n");
    out.push_str("    jmp .L_x64_rc_ret_tag_read\n\n");

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
    out.push_str("    lea alya_rodata_start(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_chk_str_buf\n");
    out.push_str("    lea alya_rodata_end(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    out.push_str(".L_x64_rc_rel_chk_str_buf:\n");
    out.push_str("    lea alya_str_buf(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_chk_tag\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    // Stable strings are immortal and never refcounted (see retain).
    out.push_str("    lea alya_str_stable(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_chk_tag\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    out.push_str(".L_x64_rc_rel_chk_tag:\n");
    // Readability probe, same contract as retain (alya-lang/alya#117).
    // Frame is 16-aligned here, so pad to keep the call aligned.
    if is_win {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        out.push_str("    push %rbx\n");
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    call alya_mem_header_readable\n");
        out.push_str("    add $40, %rsp\n");
        out.push_str("    pop %rbx\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    jz .L_x64_rc_rel_done\n");
    } else {
        out.push_str("    push %rbx\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    lea -16(%rbx), %rax\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    and $-4096, %rdi\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str("    sub %rdi, %rsi\n");
        out.push_str("    mov $1, %rdx\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    call _msync\n");
        } else {
            out.push_str("    call msync\n");
        }
        out.push_str("    add $8, %rsp\n");
        // Save the result before restoring: pop would overwrite rax.
        out.push_str("    mov %eax, %r10d\n");
        out.push_str("    pop %rbx\n");
        out.push_str("    test %r10d, %r10d\n");
        out.push_str("    jnz .L_x64_rc_rel_done\n");
    }
    // Shared tag-read entry, same arrangement as retain (see above).
    out.push_str(".L_x64_rc_rel_tag_read:\n");
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
    out.push_str("    cmp $0x5A110001, %r12\n");
    out.push_str("    jne .L_x64_rc_free_inner_map\n");
    // Arrays: release heap-kind elements (kinds 4/5/6 in the kind
    // sidecar) before freeing the buffers, or every element leaks one
    // reference (alya-lang/alya#79). Only statically-known heap kinds
    // are touched: probing unknown slots could misread a large int as a
    // pointer. Maps keep the legacy path (entries only).
    // Pad the cascade frame to a multiple of 16 so the recursive
    // fn_rc_release calls below run with an aligned stack
    // (alya-lang/alya#118). The pad sits beneath the pushed regs and
    // is removed after the pops.
    out.push_str("    sub $8, %rsp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    movq (%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    movq 24(%rbx), %r11\n");
    out.push_str("    test %r12, %r12\n");
    out.push_str("    jz .L_x64_rc_cascade_done\n");
    out.push_str("    test %r11, %r11\n");
    out.push_str("    jz .L_x64_rc_cascade_done\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x64_rc_cascade_loop:\n");
    out.push_str("    cmp %r13, %rcx\n");
    out.push_str("    jge .L_x64_rc_cascade_done\n");
    out.push_str("    movb (%r11, %rcx), %al\n");
    out.push_str("    cmp $4, %al\n");
    out.push_str("    je .L_x64_rc_cascade_rel\n");
    out.push_str("    cmp $5, %al\n");
    out.push_str("    je .L_x64_rc_cascade_rel\n");
    out.push_str("    cmp $6, %al\n");
    out.push_str("    jne .L_x64_rc_cascade_next\n");
    out.push_str(".L_x64_rc_cascade_rel:\n");
    out.push_str("    push %rcx\n");
    out.push_str("    push %r11\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    movq (%r12, %rcx, 8), %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_rc_release\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call fn_rc_release\n");
    }
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %r11\n");
    out.push_str("    pop %rcx\n");
    out.push_str(".L_x64_rc_cascade_next:\n");
    out.push_str("    inc %rcx\n");
    out.push_str("    jmp .L_x64_rc_cascade_loop\n");
    out.push_str(".L_x64_rc_cascade_done:\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    add $8, %rsp\n");
    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_rc_free_kind\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    jmp .L_x64_rc_free_kind\n");
    out.push_str(".L_x64_rc_free_inner_map:\n");
    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_rc_free_kind\n");
    // Maps: release heap-kind values (tags 4..6: array, map, struct)
    // before freeing the entries buffer (alya-lang/alya#81). Strings
    // (tag 3) and keys are deliberately untouched: strings carry no
    // refcount header, so releasing them would corrupt the heap.
    // Pad the cascade frame to a multiple of 16 so the recursive
    // fn_rc_release calls below run with an aligned stack
    // (alya-lang/alya#118: misaligned calls faulted in macOS
    // libmalloc via an aligned SSE spill). The pad is removed before
    // the pops below (order matters: it sits beneath them).
    out.push_str("    sub $8, %rsp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    movq 8(%rbx), %r13\n");
    out.push_str("    movq 16(%rbx), %r12\n");
    out.push_str("    test %r13, %r13\n");
    out.push_str("    jz .L_x64_rc_map_cascade_done\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x64_rc_map_cascade_loop:\n");
    out.push_str("    cmp %r13, %rcx\n");
    out.push_str("    jge .L_x64_rc_map_cascade_done\n");
    out.push_str("    lea (%rcx, %rcx, 2), %rax\n");
    out.push_str("    shl $3, %rax\n");
    out.push_str("    add %r12, %rax\n");
    out.push_str("    cmpl $1, 16(%rax)\n");
    out.push_str("    jne .L_x64_rc_map_cascade_next\n");
    // Release value if tag in 4..6 (array, map, struct)
    out.push_str("    movl 20(%rax), %edx\n");
    out.push_str("    cmp $4, %edx\n");
    out.push_str("    jl .L_x64_rc_map_cascade_next\n");
    out.push_str("    cmp $6, %edx\n");
    out.push_str("    jg .L_x64_rc_map_cascade_next\n");
    out.push_str("    push %rcx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %rax\n");
    out.push_str("    movq 8(%rax), %rax\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_rc_release\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call fn_rc_release\n");
    }
    out.push_str("    pop %rax\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rcx\n");
    out.push_str(".L_x64_rc_map_cascade_next:\n");
    out.push_str("    inc %rcx\n");
    out.push_str("    jmp .L_x64_rc_map_cascade_loop\n");
    out.push_str(".L_x64_rc_map_cascade_done:\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    add $8, %rsp\n");
    out.push_str("    movq 16(%rbx), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_rc_free_kind\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    jmp .L_x64_rc_free_kind\n");
    // Array kind sidecar (Phase 1, alya-lang/alya#39): freed with the
    // element buffer. Only arrays (001) carry one; maps share this
    // path but have no sidecar, so they skip to the outer free.
    out.push_str(".L_x64_rc_free_kind:\n");
    out.push_str("    cmp $0x5A110001, %r12\n");
    out.push_str("    jne .L_x64_rc_free_outer\n");
    out.push_str("    movq 24(%rbx), %rax\n");
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

    // fn_rc_release_direct(value): same contract as fn_rc_retain_direct
    // (proven heap only, no probe). Guards run first, then control joins
    // the shared tag-read tail, whose cascade keeps probed recursion.
    out.push_str(".global fn_rc_release_direct\n");
    out.push_str("fn_rc_release_direct:\n");
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
    out.push_str("    lea alya_rodata_start(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_direct_chk_str_buf\n");
    out.push_str("    lea alya_rodata_end(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    out.push_str(".L_x64_rc_rel_direct_chk_str_buf:\n");
    out.push_str("    lea alya_str_buf(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_tag_read\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    out.push_str("    lea alya_str_stable(%rip), %rax\n");
    out.push_str("    cmp %rax, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_tag_read\n");
    out.push_str("    lea 67108864(%rax), %rdx\n");
    out.push_str("    cmp %rdx, %rbx\n");
    out.push_str("    jb .L_x64_rc_rel_done\n");
    out.push_str("    jmp .L_x64_rc_rel_tag_read\n\n");

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
    out.push_str("    lea alya_rodata_start(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %rax\n");
    out.push_str("    jb .L_x64_rcc_chk_str_buf\n");
    out.push_str("    lea alya_rodata_end(%rip), %rcx\n");
    out.push_str("    cmp %rcx, %rax\n");
    out.push_str("    jb .L_x64_rcc_zero\n");
    out.push_str(".L_x64_rcc_chk_str_buf:\n");
    out.push_str("    lea alya_str_buf(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %rax\n");
    out.push_str("    jb .L_x64_rcc_chk_tag\n");
    out.push_str("    lea 67108864(%rdx), %rcx\n");
    out.push_str("    cmp %rcx, %rax\n");
    out.push_str("    jb .L_x64_rcc_zero\n");
    // Stable strings are immortal and never refcounted.
    out.push_str("    lea alya_str_stable(%rip), %rdx\n");
    out.push_str("    cmp %rdx, %rax\n");
    out.push_str("    jb .L_x64_rcc_chk_tag\n");
    out.push_str("    lea 67108864(%rdx), %rcx\n");
    out.push_str("    cmp %rcx, %rax\n");
    out.push_str("    jb .L_x64_rcc_zero\n");
    out.push_str(".L_x64_rcc_chk_tag:\n");
    if is_win {
        // alya_mem_header_readable(value): 1 = proceed, 0 = skip.
        // Save the result before restoring the value: pop would
        // overwrite rax and the test would read the pointer.
        out.push_str("    push %rax\n");
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    call alya_mem_header_readable\n");
        out.push_str("    add $40, %rsp\n");
        out.push_str("    mov %eax, %r10d\n");
        out.push_str("    pop %rax\n");
        out.push_str("    test %r10d, %r10d\n");
        out.push_str("    jz .L_x64_rcc_zero\n");
    } else {
        out.push_str("    push %rax\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    lea -16(%rax), %rax\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    and $-4096, %rdi\n");
        out.push_str("    mov 8(%rsp), %rsi\n");
        out.push_str("    sub %rdi, %rsi\n");
        out.push_str("    mov $1, %rdx\n");
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    call _msync\n");
        } else {
            out.push_str("    call msync\n");
        }
        out.push_str("    add $8, %rsp\n");
        // Save the result before restoring the value: pop would
        // overwrite rax and the test would read the pointer.
        out.push_str("    mov %eax, %r10d\n");
        out.push_str("    pop %rax\n");
        out.push_str("    test %r10d, %r10d\n");
        out.push_str("    jnz .L_x64_rcc_zero\n");
    }
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
