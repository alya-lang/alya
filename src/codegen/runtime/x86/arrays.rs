use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_array_new
    // Handle layout: +0 len, +4 cap, +8 elembuf, +12 kindbuf.
    // Element slots are 8 bytes (Phase 1 x86 port, alya-lang/alya#39):
    // ints sign-extended, floats full f64, pointers zero-high. The kind
    // sidecar holds one value-kind byte per slot (0 unknown).
    out.push_str("alya_array_new:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov %esi, %ebx\n");
    out.push_str("    cmp $8, %ebx\n");
    out.push_str("    jge .L_x86_new_cap_ok\n");
    out.push_str("    mov $8, %ebx\n");
    out.push_str("    jmp .L_x86_new_alloc_hdr\n");
    out.push_str(".L_x86_new_cap_ok:\n");
    out.push_str("    shl $1, %ebx\n");
    out.push_str(".L_x86_new_alloc_hdr:\n");
    // 28-byte header block: len, cap, data, kind, color (#63).
    out.push_str("    push $28\n");
    out.push_str("    push $1\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    movl $0x5A110001, (%eax)\n");
    out.push_str("    movl $1, 4(%eax)\n");
    out.push_str("    lea 8(%eax), %edi\n");
    out.push_str("    push $8\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, 8(%edi)\n");
    out.push_str("    push $1\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, 12(%edi)\n");
    out.push_str("    mov %ebx, %edx\n");
    out.push_str("    shl $3, %edx\n");
    out.push_str("    add %ebx, %edx\n");
    // Header block is 28 bytes (len, cap, data, kind, color) for the
    // cycle-collector color word (alya-lang/alya#63).
    out.push_str("    add $28, %edx\n");
    out.push_str("    add %edx, alya_allocated_bytes\n");
    out.push_str("    mov %esi, (%edi)\n");
    out.push_str("    mov %ebx, 4(%edi)\n");
    out.push_str("    mov %edi, %eax\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push $1\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    call alya_mem_track_alloc\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_array_push(arr, lo, hi, kind)
    // Stores the full 8-byte value with its kind byte atomically: every
    // slot write carries its kind, so kinds can never go stale. The
    // caller materializes lo/hi (ints sign-extended, floats full f64
    // bits); kind comes from static knowledge, 0 = unknown.
    // Grown regions stay zeroed (calloc/realloc-zeroed below).
    out.push_str("alya_array_push:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov 16(%ebp), %ecx\n");
    out.push_str("    mov (%esi), %ebx\n");
    out.push_str("    mov 4(%esi), %edx\n");
    out.push_str("    cmp %edx, %ebx\n");
    out.push_str("    jl .L_x86_push_store\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jnz .L_x86_push_double\n");
    out.push_str("    mov $8, %edx\n");
    out.push_str("    jmp .L_x86_push_realloc\n");
    out.push_str(".L_x86_push_double:\n");
    out.push_str("    shl $1, %edx\n");
    out.push_str(".L_x86_push_realloc:\n");
    out.push_str("    mov %edx, 4(%esi)\n");
    out.push_str("    lea (, %edx, 8), %ebx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push 8(%esi)\n");
    out.push_str("    call realloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, 8(%esi)\n");
    // Reload the new cap from memory: realloc clobbers caller-saved
    // %edx, which held it (glibc moves it; MSVCRT happened to preserve
    // it, so this only crashed on Linux).
    out.push_str("    push 4(%esi)\n");
    out.push_str("    push 12(%esi)\n");
    out.push_str("    call realloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, 12(%esi)\n");
    out.push_str("    mov 4(%esi), %ebx\n");
    out.push_str("    shl $3, %ebx\n");
    out.push_str("    add 4(%esi), %ebx\n");
    out.push_str("    add %ebx, alya_allocated_bytes\n");
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov 16(%ebp), %ecx\n");
    out.push_str(".L_x86_push_store:\n");
    out.push_str("    mov (%esi), %ebx\n");
    out.push_str("    mov 8(%esi), %edx\n");
    out.push_str("    mov %eax, (%edx, %ebx, 8)\n");
    out.push_str("    mov %ecx, 4(%edx, %ebx, 8)\n");
    out.push_str("    mov 12(%esi), %edx\n");
    out.push_str("    mov 20(%ebp), %eax\n");
    out.push_str("    mov %al, (%edx, %ebx)\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    mov %ebx, (%esi)\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_array_pop: returns lo in %eax, hi in %ecx, f64 in %xmm0,
    // slot kind in %edx (mirrors the Index-read contract below).
    out.push_str("alya_array_pop:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov (%esi), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jle alya_error_index_out_of_bounds\n");
    out.push_str("    dec %eax\n");
    out.push_str("    mov %eax, (%esi)\n");
    out.push_str("    mov 8(%esi), %ebx\n");
    out.push_str("    mov (%ebx, %eax, 8), %ecx\n");
    out.push_str("    mov %ecx, %ebx\n");
    out.push_str("    mov 8(%esi), %ecx\n");
    out.push_str("    mov 4(%ecx, %eax, 8), %ecx\n");
    out.push_str("    mov 12(%esi), %edx\n");
    out.push_str("    movzbl (%edx, %eax), %edx\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    movd %eax, %xmm0\n");
    out.push_str("    movd %ecx, %xmm1\n");
    out.push_str("    punpckldq %xmm1, %xmm0\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    pop %esi\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_print_array
    out.push_str("alya_print_array:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jnz .L_x86_arr_not_null\n");
    out.push_str("    push $alya_fmt_arr_empty\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_arr_exit\n");
    out.push_str(".L_x86_arr_not_null:\n");
    out.push_str("    mov (%esi), %edi\n");
    out.push_str("    push $alya_fmt_arr_open\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_arr_loop:\n");
    out.push_str("    cmp %edi, %ebx\n");
    out.push_str("    jge .L_x86_arr_close_call\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_arr_print_elem\n");
    out.push_str("    push $alya_fmt_arr_comma\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_arr_print_elem:\n");
    out.push_str("    mov 8(%esi), %edx\n");
    out.push_str("    push (%edx, %ebx, 8)\n");
    out.push_str("    push $alya_fmt_arr_elem\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_arr_loop\n");
    out.push_str(".L_x86_arr_close_call:\n");
    out.push_str("    push $alya_fmt_arr_close\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_arr_exit:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_args
    out.push_str(".global fn_args\n");
    out.push_str("fn_args:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov alya_argc, %eax\n");
    out.push_str("    cmp $1, %eax\n");
    out.push_str("    jg .L_x86_args_has_items\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_args_ret\n");
    out.push_str(".L_x86_args_has_items:\n");
    out.push_str("    dec %eax\n");
    out.push_str("    mov %eax, %esi\n");
    out.push_str("    push %esi\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %edi\n");
    out.push_str("    mov alya_argv, %edx\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_args_loop:\n");
    out.push_str("    cmp %esi, %ecx\n");
    out.push_str("    jge .L_x86_args_done\n");
    out.push_str("    mov 4(%edx, %ecx, 4), %eax\n");
    out.push_str("    mov 8(%edi), %ebx\n");
    out.push_str("    mov %eax, (%ebx, %ecx, 8)\n");
    out.push_str("    movl $0, 4(%ebx, %ecx, 8)\n");
    out.push_str("    mov 12(%edi), %ebx\n");
    out.push_str("    movb $3, (%ebx, %ecx)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_args_loop\n");
    out.push_str(".L_x86_args_done:\n");
    out.push_str("    mov %edi, %eax\n");
    out.push_str(".L_x86_args_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_array_slice(arr, start, end)
    out.push_str(".global alya_array_slice\n");
    out.push_str("alya_array_slice:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %ebx\n");
    out.push_str("    mov 16(%ebp), %edi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_slice_empty\n");
    out.push_str("    mov (%esi), %edx\n");
    out.push_str("    cmp $0, %ebx\n");
    out.push_str("    jge .L_x86_slice_start_ok\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_slice_start_ok:\n");
    out.push_str("    cmp $0, %edi\n");
    out.push_str("    jl .L_x86_slice_end_cap\n");
    out.push_str("    cmp %edx, %edi\n");
    out.push_str("    jle .L_x86_slice_end_ok\n");
    out.push_str(".L_x86_slice_end_cap:\n");
    out.push_str("    mov %edx, %edi\n");
    out.push_str(".L_x86_slice_end_ok:\n");
    out.push_str("    cmp %edi, %ebx\n");
    out.push_str("    jge .L_x86_slice_empty\n");
    out.push_str("    mov %edi, %edx\n");
    out.push_str("    sub %ebx, %edx\n");
    out.push_str("    push %edx\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    // Copy 8-byte values plus kind bytes. start/count spill to the
    // stack (balanced within the loop); handles stay in %esi/%edi.
    // NOTE: alya_array_new clobbers %edx, so the count is recomputed
    // after the call (the pre-call value does not survive).
    out.push_str("    mov %edi, %edx\n");
    out.push_str("    sub %ebx, %edx\n");
    out.push_str("    mov %eax, %edi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %edx\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_slice_copy_loop:\n");
    out.push_str("    mov (%esp), %edx\n");
    out.push_str("    cmp %edx, %ecx\n");
    out.push_str("    jge .L_x86_slice_copy_done\n");
    out.push_str("    mov 4(%esp), %eax\n");
    out.push_str("    add %ecx, %eax\n");
    out.push_str("    mov 8(%esi), %ebx\n");
    out.push_str("    push (%ebx, %eax, 8)\n");
    out.push_str("    mov 4(%ebx, %eax, 8), %ebx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 12(%esi), %ebx\n");
    out.push_str("    movzbl (%ebx, %eax), %ebx\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%edi), %ebx\n");
    out.push_str("    mov 12(%edi), %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    mov %al, (%edx, %ecx)\n");
    out.push_str("    pop %edx\n");
    out.push_str("    mov %edx, 4(%ebx, %ecx, 8)\n");
    out.push_str("    pop %edx\n");
    out.push_str("    mov %edx, (%ebx, %ecx, 8)\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_slice_copy_loop\n");
    out.push_str(".L_x86_slice_copy_done:\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ecx, (%edi)\n");
    out.push_str("    mov %edi, %eax\n");
    out.push_str("    jmp .L_x86_slice_done\n");
    out.push_str(".L_x86_slice_empty:\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_slice_done:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_slice
    out.push_str(".global fn_slice\n");
    out.push_str("fn_slice:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_fn_slice_null\n");
    out.push_str("    mov -8(%eax), %edx\n");
    out.push_str("    cmp $0x5A110001, %edx\n");
    out.push_str("    jne .L_x86_fn_slice_str\n");
    out.push_str("    push 16(%ebp)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_array_slice\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_fn_slice_str:\n");
    out.push_str("    mov 16(%ebp), %edx\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    cmp $0, %edx\n");
    out.push_str("    jl .L_x86_fn_slice_str_call\n");
    out.push_str("    sub %ecx, %edx\n");
    out.push_str("    cmp $0, %edx\n");
    out.push_str("    jge .L_x86_fn_slice_str_call\n");
    out.push_str("    xor %edx, %edx\n");
    out.push_str(".L_x86_fn_slice_str_call:\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_substring\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_fn_slice_null:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
