use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_map_hash
    out.push_str("alya_map_hash:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    cmp $256, %edx\n");
    out.push_str("    jb .L_x86_mhash_int\n");
    out.push_str("    mov $5381, %eax\n");
    out.push_str(".L_x86_mhash_loop:\n");
    out.push_str("    movzbl (%edx), %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_mhash_done\n");
    out.push_str("    mov %eax, %ebx\n");
    out.push_str("    shl $5, %ebx\n");
    out.push_str("    add %ebx, %eax\n");
    out.push_str("    add %ecx, %eax\n");
    out.push_str("    inc %edx\n");
    out.push_str("    jmp .L_x86_mhash_loop\n");
    out.push_str(".L_x86_mhash_int:\n");
    out.push_str("    mov %edx, %eax\n");
    out.push_str(".L_x86_mhash_done:\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_map_key_eq / fn_streq
    out.push_str(".global fn_streq\n");
    out.push_str("fn_streq:\n");
    out.push_str(".global alya_map_key_eq\n");
    out.push_str("alya_map_key_eq:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    cmp %esi, %edi\n");
    out.push_str("    je .L_x86_mkeq_true\n");
    out.push_str("    cmp $65536, %esi\n");
    out.push_str("    jb .L_x86_mkeq_false\n");
    out.push_str("    cmp $65536, %edi\n");
    out.push_str("    jb .L_x86_mkeq_false\n");
    out.push_str(".L_x86_mkeq_str:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    movb (%edi), %cl\n");
    out.push_str("    cmp %al, %cl\n");
    out.push_str("    jne .L_x86_mkeq_false\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_mkeq_true\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_mkeq_str\n");
    out.push_str(".L_x86_mkeq_true:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_mkeq_end\n");
    out.push_str(".L_x86_mkeq_false:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_mkeq_end:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_strcmp
    out.push_str(".global fn_strcmp\n");
    out.push_str("fn_strcmp:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    cmp %esi, %edi\n");
    out.push_str("    je .L_x86_strcmp_eq\n");
    out.push_str("    cmp $65536, %esi\n");
    out.push_str("    jb .L_x86_strcmp_s1_null\n");
    out.push_str("    cmp $65536, %edi\n");
    out.push_str("    jb .L_x86_strcmp_s2_null\n");
    out.push_str(".L_x86_strcmp_loop:\n");
    out.push_str("    movzbl (%esi), %eax\n");
    out.push_str("    movzbl (%edi), %ecx\n");
    out.push_str("    cmp %al, %cl\n");
    out.push_str("    jne .L_x86_strcmp_diff\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_strcmp_eq\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_strcmp_loop\n");
    out.push_str(".L_x86_strcmp_diff:\n");
    out.push_str("    sub %ecx, %eax\n");
    out.push_str("    jmp .L_x86_strcmp_end\n");
    out.push_str(".L_x86_strcmp_s1_null:\n");
    out.push_str("    cmp $65536, %edi\n");
    out.push_str("    jb .L_x86_strcmp_eq\n");
    out.push_str("    mov $-1, %eax\n");
    out.push_str("    jmp .L_x86_strcmp_end\n");
    out.push_str(".L_x86_strcmp_s2_null:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_strcmp_end\n");
    out.push_str(".L_x86_strcmp_eq:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_strcmp_end:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_map
    // Entry layout (Phase 1 x86 port, alya-lang/alya#39), 20 bytes:
    // +0 value lo, +4 value hi (8-byte slots: ints sign-extended,
    // floats full f64), +8 key, +12 state (0 empty, 1 live,
    // 2 tombstone), +16 value-kind tag (0 unknown).
    // Value first keeps the f64 slot 4-byte aligned.
    out.push_str("fn_map:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push $20\n");
    out.push_str("    push $1\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    movl $0x5A110002, (%eax)\n");
    out.push_str("    movl $1, 4(%eax)\n");
    out.push_str("    lea 8(%eax), %ebx\n"); // ebx = map header
    out.push_str("    movl $0, (%ebx)\n"); // len = 0
    out.push_str("    movl $64, 4(%ebx)\n"); // cap = 64
    out.push_str("    push $20\n");
    out.push_str("    push $64\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, 8(%ebx)\n"); // entries
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    addl $1300, alya_allocated_bytes\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $0\n");
    out.push_str("    push $2\n");
    out.push_str("    push $1300\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_mem_track_alloc\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_set(map, key, lo, hi, kind)
    // Stores the full 8-byte value with its kind tag atomically on both
    // insert and overwrite paths. Callers materialize lo/hi (ints
    // sign-extended, floats full f64); kind 0 = unknown.
    out.push_str("fn_set:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $20, %esp\n"); // -16: first_tombstone, -20: slot, -24: mask, -28: count, -32: rehash index
    out.push_str("    mov 8(%ebp), %esi\n"); // esi = map
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_set_done\n");
    // check resize: if len * 2 >= cap
    out.push_str("    mov (%esi), %eax\n");
    out.push_str("    shl $1, %eax\n");
    out.push_str("    cmp 4(%esi), %eax\n");
    out.push_str("    jl .L_x86_set_no_resize\n");
    // Resize table
    out.push_str("    mov 4(%esi), %eax\n");
    out.push_str("    shl $1, %eax\n"); // new_cap
    out.push_str("    mov %eax, %ebx\n"); // ebx = new_cap
    out.push_str("    push $20\n");
    out.push_str("    push %ebx\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %eax, %edi\n"); // edi = new_entries
    out.push_str("    imul $20, %ebx, %edx\n");
    out.push_str("    add %edx, alya_allocated_bytes\n");
    out.push_str("    lea -1(%ebx), %edx\n"); // edx = new_mask
    out.push_str("    mov %edx, -24(%ebp)\n"); // spill: probe clobbers %edx
                                              // rehash old entries
    out.push_str("    mov 8(%esi), %ecx\n"); // ecx = old_entries
    out.push_str("    movl $0, -32(%ebp)\n"); // i = 0
    out.push_str(".L_x86_rehash_loop:\n");
    out.push_str("    mov -32(%ebp), %eax\n");
    out.push_str("    cmp 4(%esi), %eax\n");
    out.push_str("    jge .L_x86_rehash_done\n");
    out.push_str("    imul $20, %eax, %ebx\n");
    out.push_str("    add %ecx, %ebx\n"); // ebx = &old_entries[i]
    out.push_str("    cmpl $1, 12(%ebx)\n");
    out.push_str("    jne .L_x86_rehash_next\n");
    // hash old key (spill caller-saves around the call)
    out.push_str("    push %eax\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%ebx)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %ecx\n");
    out.push_str("    pop %eax\n"); // eax = i again
    out.push_str("    and %edx, %eax\n"); // new_slot in eax
    out.push_str("    push %eax\n"); // spill slot; probe uses (%esp)
    out.push_str(".L_x86_rehash_probe:\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    imul $20, %eax, %eax\n");
    out.push_str("    add %edi, %eax\n"); // eax = &new_entries[slot]
    out.push_str("    cmpl $0, 12(%eax)\n");
    out.push_str("    je .L_x86_rehash_put\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    mov %eax, (%esp)\n");
    out.push_str("    jmp .L_x86_rehash_probe\n");
    out.push_str(".L_x86_rehash_put:\n");
    out.push_str("    pop %eax\n"); // slot
    out.push_str("    imul $20, %eax, %eax\n");
    out.push_str("    add %edi, %eax\n"); // eax = &new_entries[slot]
    out.push_str("    mov -32(%ebp), %ebx\n");
    out.push_str("    imul $20, %ebx, %ebx\n");
    out.push_str("    add %ecx, %ebx\n"); // ebx = &old_entries[i]
    out.push_str("    mov (%ebx), %edx\n");
    out.push_str("    mov %edx, (%eax)\n"); // val lo
    out.push_str("    mov 4(%ebx), %edx\n");
    out.push_str("    mov %edx, 4(%eax)\n"); // val hi
    out.push_str("    mov 8(%ebx), %edx\n");
    out.push_str("    mov %edx, 8(%eax)\n"); // key
    out.push_str("    movl $1, 12(%eax)\n"); // state = live
    out.push_str("    mov 16(%ebx), %edx\n");
    out.push_str("    mov %edx, 16(%eax)\n"); // tag carried over
    out.push_str("    mov -24(%ebp), %edx\n"); // restore mask
    out.push_str(".L_x86_rehash_next:\n");
    out.push_str("    incl -32(%ebp)\n");
    out.push_str("    jmp .L_x86_rehash_loop\n");
    out.push_str(".L_x86_rehash_done:\n");
    out.push_str("    mov 4(%esi), %eax\n");
    out.push_str("    shl $1, %eax\n");
    out.push_str("    mov %eax, 4(%esi)\n"); // cap = new_cap
    out.push_str("    mov %edi, 8(%esi)\n"); // entries = new_entries
    out.push_str(".L_x86_set_no_resize:\n");
    out.push_str("    mov 4(%esi), %eax\n");
    out.push_str("    dec %eax\n");
    out.push_str("    mov %eax, -24(%ebp)\n"); // mask
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    and -24(%ebp), %eax\n");
    out.push_str("    mov %eax, -20(%ebp)\n"); // slot
    out.push_str("    movl $-1, -16(%ebp)\n"); // first_tombstone = -1
    out.push_str("    movl $0, -28(%ebp)\n"); // count = 0
    out.push_str(".L_x86_set_probe:\n");
    out.push_str("    mov -28(%ebp), %eax\n");
    out.push_str("    cmp 4(%esi), %eax\n");
    out.push_str("    jge .L_x86_set_use_tomb\n");
    out.push_str("    mov -20(%ebp), %eax\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n"); // edi = &entries[slot]
    out.push_str("    mov 12(%edi), %eax\n"); // flag
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_set_empty\n");
    out.push_str("    cmp $2, %eax\n");
    out.push_str("    jne .L_x86_set_check_key\n");
    out.push_str("    cmpl $-1, -16(%ebp)\n");
    out.push_str("    jne .L_x86_set_next\n");
    out.push_str("    mov -20(%ebp), %eax\n");
    out.push_str("    mov %eax, -16(%ebp)\n");
    out.push_str("    jmp .L_x86_set_next\n");
    out.push_str(".L_x86_set_check_key:\n");
    out.push_str("    push 8(%edi)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_set_next\n");
    // Key match: overwrite value and tag.
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    mov %eax, (%edi)\n");
    out.push_str("    mov 20(%ebp), %eax\n");
    out.push_str("    mov %eax, 4(%edi)\n");
    out.push_str("    mov 24(%ebp), %eax\n");
    out.push_str("    mov %al, 16(%edi)\n");
    out.push_str("    jmp .L_x86_set_done\n");
    out.push_str(".L_x86_set_next:\n");
    out.push_str("    incl -28(%ebp)\n");
    out.push_str("    mov -20(%ebp), %eax\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and -24(%ebp), %eax\n");
    out.push_str("    mov %eax, -20(%ebp)\n");
    out.push_str("    jmp .L_x86_set_probe\n");
    out.push_str(".L_x86_set_empty:\n");
    out.push_str("    cmpl $-1, -16(%ebp)\n");
    out.push_str("    je .L_x86_set_insert_here\n");
    out.push_str(".L_x86_set_use_tomb:\n");
    out.push_str("    cmpl $-1, -16(%ebp)\n");
    out.push_str("    je .L_x86_set_done\n");
    out.push_str("    mov -16(%ebp), %eax\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n");
    out.push_str(".L_x86_set_insert_here:\n");
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov %eax, 8(%edi)\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    mov %eax, (%edi)\n");
    out.push_str("    mov 20(%ebp), %eax\n");
    out.push_str("    mov %eax, 4(%edi)\n");
    out.push_str("    mov 24(%ebp), %eax\n");
    out.push_str("    mov %al, 16(%edi)\n");
    out.push_str("    movl $1, 12(%edi)\n");
    out.push_str("    incl (%esi)\n"); // len++
    out.push_str(".L_x86_set_done:\n");
    out.push_str("    add $20, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_get
    out.push_str("fn_get:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n"); // esi = map
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_get_not_found\n");
    out.push_str("    cmp $65536, %esi\n");
    out.push_str("    jb .L_x86_get_not_found\n");
    out.push_str("    movl -8(%esi), %eax\n");
    out.push_str("    cmpl $0x5A110001, %eax\n");
    out.push_str("    je .L_x86_get_array\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov 4(%esi), %edx\n"); // cap
    out.push_str("    dec %edx\n"); // mask
    out.push_str("    and %edx, %eax\n"); // slot
    out.push_str("    xor %ebx, %ebx\n"); // count
    out.push_str(".L_x86_get_loop:\n");
    out.push_str("    cmp 4(%esi), %ebx\n");
    out.push_str("    jge .L_x86_get_not_found\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n"); // edi = &entries[slot]
    out.push_str("    cmpl $0, 12(%edi)\n");
    out.push_str("    je .L_x86_get_not_found\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_get_next\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%edi)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jnz .L_x86_get_found\n");
    out.push_str(".L_x86_get_next:\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_get_loop\n");
    out.push_str(".L_x86_get_found:\n");
    // Contract (Phase 1 x86 port): lo in %eax, hi in %ecx, f64 in
    // %xmm0, entry kind tag in %edx. Mirrors x64 (%rax/%edx).
    out.push_str("    mov (%edi), %eax\n");
    out.push_str("    mov 4(%edi), %ecx\n");
    out.push_str("    movl 16(%edi), %edx\n");
    out.push_str("    movd %eax, %xmm0\n");
    out.push_str("    movd %ecx, %xmm1\n");
    out.push_str("    punpckldq %xmm1, %xmm0\n");
    out.push_str("    jmp .L_x86_get_ret\n");
    out.push_str(".L_x86_get_array:\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    js .L_x86_get_not_found\n");
    out.push_str("    cmpl (%esi), %edx\n");
    out.push_str("    jae .L_x86_get_not_found\n");
    out.push_str("    mov 8(%esi), %eax\n");
    out.push_str("    mov (%eax, %edx, 8), %ebx\n");
    out.push_str("    mov 4(%eax, %edx, 8), %ecx\n");
    out.push_str("    mov 12(%esi), %eax\n");
    out.push_str("    movzbl (%eax, %edx), %edx\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    movd %eax, %xmm0\n");
    out.push_str("    movd %ecx, %xmm1\n");
    out.push_str("    punpckldq %xmm1, %xmm0\n");
    out.push_str("    jmp .L_x86_get_ret\n");
    out.push_str(".L_x86_get_not_found:\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str("    xor %edx, %edx\n");
    out.push_str(".L_x86_get_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_map_set_tag(map, key, tag): records a value-kind tag in the
    // entry state word's neighbor slot (+16). Tags: 0 unknown, 1 int,
    // 2 float, 3 string, 4 array, 5 map, 6 struct. Misses are no-ops.
    // Returns 1 when stored, 0 otherwise. Mirrors x64 (high 32 bits of
    // the state word there; a dedicated slot here — entries are 20B).
    out.push_str(".global fn_map_set_tag\n");
    out.push_str("fn_map_set_tag:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_set_tag_ret\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov 4(%esi), %edx\n");
    out.push_str("    dec %edx\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_set_tag_loop:\n");
    out.push_str("    cmp 4(%esi), %ebx\n");
    out.push_str("    jge .L_x86_set_tag_ret\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n");
    out.push_str("    cmpl $0, 12(%edi)\n");
    out.push_str("    je .L_x86_set_tag_ret\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_set_tag_next\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%edi)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jnz .L_x86_set_tag_found\n");
    out.push_str(".L_x86_set_tag_next:\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_set_tag_loop\n");
    out.push_str(".L_x86_set_tag_found:\n");
    out.push_str("    mov 16(%ebp), %eax\n");
    out.push_str("    mov %al, 16(%edi)\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str(".L_x86_set_tag_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_has
    out.push_str("fn_has:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_has_ret\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov 4(%esi), %edx\n");
    out.push_str("    dec %edx\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_has_loop:\n");
    out.push_str("    cmp 4(%esi), %ebx\n");
    out.push_str("    jge .L_x86_has_not_found\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n");
    out.push_str("    cmpl $0, 12(%edi)\n");
    out.push_str("    je .L_x86_has_not_found\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_has_next\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%edi)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jnz .L_x86_has_found\n");
    out.push_str(".L_x86_has_next:\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_has_loop\n");
    out.push_str(".L_x86_has_found:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_has_ret\n");
    out.push_str(".L_x86_has_not_found:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_has_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_remove
    out.push_str("fn_remove:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_rem_ret\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_hash\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov 4(%esi), %edx\n");
    out.push_str("    dec %edx\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_rem_loop:\n");
    out.push_str("    cmp 4(%esi), %ebx\n");
    out.push_str("    jge .L_x86_rem_not_found\n");
    out.push_str("    imul $20, %eax, %edi\n");
    out.push_str("    add 8(%esi), %edi\n");
    out.push_str("    cmpl $0, 12(%edi)\n");
    out.push_str("    je .L_x86_rem_not_found\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_rem_next\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push 8(%edi)\n");
    out.push_str("    push 12(%ebp)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    pop %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jnz .L_x86_rem_found\n");
    out.push_str(".L_x86_rem_next:\n");
    out.push_str("    inc %eax\n");
    out.push_str("    and %edx, %eax\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_rem_loop\n");
    out.push_str(".L_x86_rem_found:\n");
    out.push_str("    movl $2, 12(%edi)\n"); // tombstone
    out.push_str("    decl (%esi)\n"); // len--
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_rem_ret\n");
    out.push_str(".L_x86_rem_not_found:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_rem_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_keys
    out.push_str("fn_keys:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %ebx\n"); // ebx = array
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_keys_done\n");
    out.push_str("    mov 8(%esi), %edi\n"); // edi = entries
    out.push_str("    xor %ecx, %ecx\n"); // ecx = i
    out.push_str(".L_x86_keys_loop:\n");
    out.push_str("    cmp 4(%esi), %ecx\n");
    out.push_str("    jge .L_x86_keys_done\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_keys_next\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push $0\n"); // kind = unknown (fallback classifies pointers)
    out.push_str("    push $0\n"); // hi
    out.push_str("    push 8(%edi)\n"); // lo = key
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %ecx\n");
    out.push_str(".L_x86_keys_next:\n");
    out.push_str("    add $20, %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_keys_loop\n");
    out.push_str(".L_x86_keys_done:\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_values
    out.push_str("fn_values:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %ebx\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_vals_done\n");
    out.push_str("    mov 8(%esi), %edi\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_vals_loop:\n");
    out.push_str("    cmp 4(%esi), %ecx\n");
    out.push_str("    jge .L_x86_vals_done\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_vals_next\n");
    // Push the full 8-byte value with its entry tag: values arrays
    // carry exact kinds, so `say`/dispatch needs no fallback.
    out.push_str("    push %ecx\n");
    out.push_str("    movzbl 16(%edi), %eax\n");
    out.push_str("    push %eax\n"); // kind
    out.push_str("    push 4(%edi)\n"); // hi
    out.push_str("    push (%edi)\n"); // lo
    out.push_str("    push %ebx\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %ecx\n");
    out.push_str(".L_x86_vals_next:\n");
    out.push_str("    add $20, %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_vals_loop\n");
    out.push_str(".L_x86_vals_done:\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_print_map
    out.push_str("alya_print_map:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jnz .L_x86_pmap_not_null\n");
    out.push_str("    push $alya_fmt_map_null\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_pmap_exit\n");
    out.push_str(".L_x86_pmap_not_null:\n");
    out.push_str("    cmpl $0, (%esi)\n");
    out.push_str("    jne .L_x86_pmap_has_items\n");
    out.push_str("    push $alya_fmt_map_empty\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_pmap_exit\n");
    out.push_str(".L_x86_pmap_has_items:\n");
    out.push_str("    push $alya_fmt_map_open\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    xor %ebx, %ebx\n"); // printed = 0
    out.push_str("    mov 8(%esi), %edi\n"); // edi = entries
    out.push_str("    xor %ecx, %ecx\n"); // ecx = slot
    out.push_str(".L_x86_pmap_loop:\n");
    out.push_str("    cmp 4(%esi), %ecx\n");
    out.push_str("    jge .L_x86_pmap_close\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str("    jne .L_x86_pmap_next\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_pmap_print_pair\n");
    out.push_str("    push %ecx\n");
    out.push_str("    push $alya_fmt_arr_comma\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    pop %ecx\n");
    out.push_str(".L_x86_pmap_print_pair:\n");
    out.push_str("    push %ecx\n");
    out.push_str("    mov 8(%edi), %eax\n"); // key
    out.push_str("    cmp $256, %eax\n");
    out.push_str("    jb .L_x86_pmap_key_num\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_prompt\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    jmp .L_x86_pmap_colon\n");
    out.push_str(".L_x86_pmap_key_num:\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_arr_elem\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str(".L_x86_pmap_colon:\n");
    out.push_str("    push $alya_fmt_map_colon\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov (%edi), %eax\n"); // val lo (print path is lo-word)
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_arr_elem\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    pop %ecx\n");
    out.push_str("    inc %ebx\n");
    out.push_str(".L_x86_pmap_next:\n");
    out.push_str("    add $20, %edi\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_pmap_loop\n");
    out.push_str(".L_x86_pmap_close:\n");
    out.push_str("    push $alya_fmt_map_close\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_pmap_exit:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_in
    out.push_str(".global fn_in\n");
    out.push_str("fn_in:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_in_ret\n");
    out.push_str("    test $3, %esi\n");
    out.push_str("    jnz .L_x86_in_str\n");
    out.push_str("    cmp $65536, %esi\n");
    out.push_str("    jb .L_x86_in_str\n");
    out.push_str("    cmpl $0x5A110002, -8(%esi)\n");
    out.push_str("    je .L_x86_in_map\n");
    out.push_str("    cmpl $0x5A110001, -8(%esi)\n");
    out.push_str("    je .L_x86_in_arr\n");
    out.push_str(".L_x86_in_str:\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    call fn_contains\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    jmp .L_x86_in_ret\n");
    out.push_str(".L_x86_in_map:\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    call fn_has\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    jmp .L_x86_in_ret\n");
    out.push_str(".L_x86_in_arr:\n");
    out.push_str("    mov (%esi), %edx\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_in_arr_loop:\n");
    out.push_str("    cmp %edx, %ebx\n");
    out.push_str("    jge .L_x86_in_fail\n");
    out.push_str("    mov 8(%esi), %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push (%eax, %ebx, 8)\n");
    out.push_str("    call alya_map_key_eq\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    pop %edx\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_in_found\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_in_arr_loop\n");
    out.push_str(".L_x86_in_found:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_in_ret\n");
    out.push_str(".L_x86_in_fail:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_in_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
