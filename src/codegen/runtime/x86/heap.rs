use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_alloc
    out.push_str(".global fn_alloc\n");
    out.push_str("fn_alloc:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    add %eax, alya_allocated_bytes\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}malloc\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_alloc_done\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push $5\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_mem_track_alloc\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str(".L_x86_alloc_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_free
    out.push_str(".global fn_free\n");
    out.push_str("fn_free:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_free_done\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call alya_mem_track_free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_free_done:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_realloc
    out.push_str(".global fn_realloc\n");
    out.push_str("fn_realloc:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    add %eax, alya_allocated_bytes\n");
    out.push_str("    push %eax\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call realloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_copy_mem
    out.push_str(".global fn_copy_mem\n");
    out.push_str("fn_copy_mem:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push 16(%ebp)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call memcpy\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_zero_mem
    out.push_str(".global fn_zero_mem\n");
    out.push_str("fn_zero_mem:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    push $0\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call memset\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_peek_byte
    out.push_str(".global fn_peek_byte\n");
    out.push_str("fn_peek_byte:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    movzbl (%edx), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_poke_byte
    out.push_str(".global fn_poke_byte\n");
    out.push_str("fn_poke_byte:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    movb %al, (%edx)\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_peek_int
    out.push_str(".global fn_peek_int\n");
    out.push_str("fn_peek_int:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    mov (%edx), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_poke_int
    out.push_str(".global fn_poke_int\n");
    out.push_str("fn_poke_int:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    mov %eax, (%edx)\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_poke_float(ptr, offset, dbl): 8-byte sibling of fn_poke_int for
    // float values (e.g. `write_float`, which delegates to `poke_int`).
    // The value arrives as an 8-byte double at 16(%ebp) (see
    // `x86_native_float_args`); the int entry would store only the low
    // word.
    out.push_str(".global fn_poke_float\n");
    out.push_str("fn_poke_float:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    movsd 16(%ebp), %xmm0\n");
    out.push_str("    movsd %xmm0, (%edx)\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_mem_peek_f32(ptr, offset) -> float
    // Loads a 32-bit float, widens to f64 in %xmm0, and returns it through
    // the x86 float convention (low 32 bits of the f64 pattern in %eax,
    // full value in %xmm0) exactly like fn_simd_f64x4_get.
    out.push_str(".global fn_mem_peek_f32\n");
    out.push_str("fn_mem_peek_f32:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    movss (%edx), %xmm0\n");
    out.push_str("    cvtss2sd %xmm0, %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_f32(ptr, offset, val) -> void
    // Float arguments arrive as a stack double (callers push 8 bytes;
    // %xmm0 does not survive argument evaluation); narrow to 32-bit
    // storage with hardware rounding.
    out.push_str(".global fn_mem_poke_f32\n");
    out.push_str("fn_mem_poke_f32:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    movsd 16(%ebp), %xmm0\n");
    out.push_str("    cvtsd2ss %xmm0, %xmm0\n");
    out.push_str("    movss %xmm0, (%edx)\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_mem_peek_i32(ptr, offset) -> int
    // x86 value slots are 32-bit, identical to peek_int.
    out.push_str(".global fn_mem_peek_i32\n");
    out.push_str("fn_mem_peek_i32:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    mov (%edx), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_mem_poke_i32(ptr, offset, val) -> void
    out.push_str(".global fn_mem_poke_i32\n");
    out.push_str("fn_mem_poke_i32:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    add %ecx, %edx\n");
    out.push_str("    mov %eax, (%edx)\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_str_from_ptr
    out.push_str(".global fn_str_from_ptr\n");
    out.push_str("fn_str_from_ptr:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_strfromptr_ret\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_strfromptr_ret:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_str_to_ptr
    out.push_str(".global fn_str_to_ptr\n");
    out.push_str("fn_str_to_ptr:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_mem_allocated
    out.push_str(".global fn_mem_allocated\n");
    out.push_str("fn_mem_allocated:\n");
    out.push_str("    mov alya_allocated_bytes, %eax\n");
    out.push_str("    ret\n\n");

    // fn_mem_reset_alloc
    out.push_str(".global fn_mem_reset_alloc\n");
    out.push_str("fn_mem_reset_alloc:\n");
    out.push_str("    movl $0, alya_allocated_bytes\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    ret\n\n");

    // fn_str_clone
    out.push_str(".global fn_str_clone\n");
    out.push_str("fn_str_clone:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_sclone_empty\n");
    out.push_str("    mov %esi, %edi\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_sclone_len:\n");
    out.push_str("    cmpb $0, (%edi)\n");
    out.push_str("    je .L_x86_sclone_alloc\n");
    out.push_str("    inc %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_sclone_len\n");
    out.push_str(".L_x86_sclone_alloc:\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    mov %ecx, %ebx\n");
    out.push_str("    add %ebx, alya_allocated_bytes\n");
    out.push_str("    push %ebx\n");
    out.push_str(&format!("    call {}malloc\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_sclone_empty\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}memcpy\n", p));
    out.push_str("    add $12, %esp\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push $4\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_mem_track_alloc\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jmp .L_x86_sclone_done\n");
    out.push_str(".L_x86_sclone_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_sclone_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_str_free
    out.push_str(".global fn_str_free\n");
    out.push_str("fn_str_free:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_sfree_done\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call alya_mem_track_free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_sfree_done:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_rc_retain
    out.push_str(".global fn_rc_retain\n");
    out.push_str("fn_rc_retain:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test $3, %eax\n");
    out.push_str("    jnz .L_x86_rc_retain_done\n");
    out.push_str("    cmp $65536, %eax\n");
    out.push_str("    jbe .L_x86_rc_retain_done\n");
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str("    jae .L_x86_rc_retain_done\n");
    out.push_str("    movl -8(%eax), %edx\n");
    out.push_str("    cmpl $0x5A110001, %edx\n");
    out.push_str("    je .L_x86_rc_retain_ok\n");
    out.push_str("    cmpl $0x5A110002, %edx\n");
    out.push_str("    je .L_x86_rc_retain_ok\n");
    out.push_str("    cmpl $0x5A110003, %edx\n");
    out.push_str("    jne .L_x86_rc_retain_done\n");
    out.push_str(".L_x86_rc_retain_ok:\n");
    out.push_str("    lock incl -4(%eax)\n");
    out.push_str(".L_x86_rc_retain_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_rc_release
    out.push_str(".global fn_rc_release\n");
    out.push_str("fn_rc_release:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test $3, %ebx\n");
    out.push_str("    jnz .L_x86_rc_rel_done\n");
    out.push_str("    cmp $65536, %ebx\n");
    out.push_str("    jbe .L_x86_rc_rel_done\n");
    out.push_str("    cmpl $0xC0000000, %ebx\n");
    out.push_str("    jae .L_x86_rc_rel_done\n");
    out.push_str("    movl -8(%ebx), %esi\n");
    out.push_str("    cmpl $0x5A110001, %esi\n");
    out.push_str("    je .L_x86_rc_rel_ok\n");
    out.push_str("    cmpl $0x5A110002, %esi\n");
    out.push_str("    je .L_x86_rc_rel_ok\n");
    out.push_str("    cmpl $0x5A110003, %esi\n");
    out.push_str("    jne .L_x86_rc_rel_done\n");
    out.push_str(".L_x86_rc_rel_ok:\n");
    out.push_str("    lock decl -4(%ebx)\n");
    out.push_str("    jnz .L_x86_rc_rel_purple\n");
    // Freed: clear the cycle-collector color (fat pointers carry none).
    // %esi still holds the magic word (alya-lang/alya#63).
    out.push_str("    cmpl $0x5A110004, %esi\n");
    out.push_str("    je .L_x86_rc_rel_nozero\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_gc_color_addr\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movl $0, (%eax)\n");
    out.push_str(".L_x86_rc_rel_nozero:\n");
    out.push_str("    cmpl $0x5A110001, %esi\n");
    out.push_str("    je .L_x86_rc_free_inner\n");
    out.push_str("    cmpl $0x5A110002, %esi\n");
    out.push_str("    je .L_x86_rc_free_inner\n");
    out.push_str("    jmp .L_x86_rc_free_outer\n");
    out.push_str(".L_x86_rc_free_inner:\n");
    out.push_str("    cmpl $0x5A110001, %esi\n");
    out.push_str("    jne .L_x86_rc_free_inner_map\n");
    // Arrays: release heap-kind elements (kinds 4/5/6 in the kind
    // sidecar) before freeing the buffers, or every element leaks one
    // reference (alya-lang/alya#79). Only statically-known heap kinds
    // are touched: probing unknown slots could misread a large int as a
    // pointer. Maps keep the legacy path (entries only).
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    movl (%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    movl 12(%ebx), %edx\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_rc_cascade_done\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_rc_cascade_done\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_rc_cascade_loop:\n");
    out.push_str("    cmpl %edi, %ecx\n");
    out.push_str("    jge .L_x86_rc_cascade_done\n");
    out.push_str("    movb (%edx, %ecx), %al\n");
    out.push_str("    cmpb $4, %al\n");
    out.push_str("    je .L_x86_rc_cascade_rel\n");
    out.push_str("    cmpb $5, %al\n");
    out.push_str("    je .L_x86_rc_cascade_rel\n");
    out.push_str("    cmpb $6, %al\n");
    out.push_str("    jne .L_x86_rc_cascade_next\n");
    out.push_str(".L_x86_rc_cascade_rel:\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    movl (%esi, %ecx, 8), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %ecx\n");
    out.push_str(".L_x86_rc_cascade_next:\n");
    out.push_str("    incl %ecx\n");
    out.push_str("    jmp .L_x86_rc_cascade_loop\n");
    out.push_str(".L_x86_rc_cascade_done:\n");
    out.push_str("    movl 12(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_rc_free_elembuf\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_rc_free_elembuf:\n");
    out.push_str("    movl 8(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_rc_cascade_restore\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_rc_cascade_restore:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    jmp .L_x86_rc_free_outer\n");
    out.push_str(".L_x86_rc_free_inner_map:\n");
    out.push_str("    movl 8(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_rc_free_outer\n");
    // Maps: release heap-kind values (tags 4..6: array, map, struct)
    // before freeing the entries buffer (alya-lang/alya#81). Strings
    // (tag 3) and keys are deliberately untouched: strings carry no
    // refcount header, so releasing them would corrupt the heap.
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    movl 4(%ebx), %edi\n");
    out.push_str("    movl 8(%ebx), %esi\n");
    out.push_str("    test %edi, %edi\n");
    out.push_str("    jz .L_x86_rc_map_cascade_done\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_rc_map_cascade_loop:\n");
    out.push_str("    cmpl %edi, %ecx\n");
    out.push_str("    jge .L_x86_rc_map_cascade_done\n");
    out.push_str("    imul $20, %ecx, %eax\n");
    out.push_str("    add %esi, %eax\n");
    out.push_str("    cmpl $1, 12(%eax)\n");
    out.push_str("    jne .L_x86_rc_map_cascade_next\n");
    // Release value if tag in 4..6 (array, map, struct)
    out.push_str("    movzbl 16(%eax), %edx\n");
    out.push_str("    cmp $4, %edx\n");
    out.push_str("    jl .L_x86_rc_map_cascade_next\n");
    out.push_str("    cmp $6, %edx\n");
    out.push_str("    jg .L_x86_rc_map_cascade_next\n");
    // Release value
    out.push_str("    push %ecx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    push (%eax)\n");
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ecx\n");
    out.push_str(".L_x86_rc_map_cascade_next:\n");
    out.push_str("    incl %ecx\n");
    out.push_str("    jmp .L_x86_rc_map_cascade_loop\n");
    out.push_str(".L_x86_rc_map_cascade_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    movl 8(%ebx), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_rc_free_outer\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_rc_free_outer:\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_mem_track_free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    lea -8(%ebx), %eax\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_rc_rel_done\n");
    // Decremented but alive: candidate for cycle collection (fat
    // pointers carry no color word; alya-lang/alya#63).
    out.push_str(".L_x86_rc_rel_purple:\n");
    out.push_str("    cmpl $0x5A110004, %esi\n");
    out.push_str("    je .L_x86_rc_rel_done\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call fn_gc_add_purple\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_rc_rel_done:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_rc_count
    out.push_str(".global fn_rc_count\n");
    out.push_str("fn_rc_count:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test $3, %eax\n");
    out.push_str("    jnz .L_x86_rcc_zero\n");
    out.push_str("    cmp $65536, %eax\n");
    out.push_str("    jbe .L_x86_rcc_zero\n");
    out.push_str("    cmpl $0xC0000000, %eax\n");
    out.push_str("    jae .L_x86_rcc_zero\n");
    out.push_str("    movl -8(%eax), %edx\n");
    out.push_str("    cmpl $0x5A110001, %edx\n");
    out.push_str("    je .L_x86_rcc_ok\n");
    out.push_str("    cmpl $0x5A110002, %edx\n");
    out.push_str("    je .L_x86_rcc_ok\n");
    out.push_str("    cmpl $0x5A110003, %edx\n");
    out.push_str("    je .L_x86_rcc_ok\n");
    out.push_str(".L_x86_rcc_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_rcc_ok:\n");
    out.push_str("    movl -4(%eax), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
