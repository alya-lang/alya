use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // alya_struct_new
    out.push_str("alya_struct_new:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %ebx\n");
    out.push_str("    lea 3(%ebx), %eax\n");
    out.push_str("    push $4\n");
    out.push_str("    push %eax\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebx, %edx\n");
    out.push_str("    add $3, %edx\n");
    out.push_str("    shl $2, %edx\n");
    out.push_str("    add %edx, alya_allocated_bytes\n");
    out.push_str("    movl $0x5A110003, (%eax)\n");
    out.push_str("    movl $1, 4(%eax)\n");
    out.push_str("    lea 8(%eax), %eax\n");
    out.push_str("    mov %esi, (%eax)\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %esi\n");
    out.push_str("    push $3\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_mem_track_alloc\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_print_struct
    out.push_str("alya_print_struct:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jnz .L_x86_struct_not_null\n");
    out.push_str("    push $alya_fmt_struct_null\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_struct_exit\n");
    out.push_str(".L_x86_struct_not_null:\n");
    out.push_str("    mov (%esi), %edi\n");
    out.push_str("    push (%edi)\n");
    out.push_str("    push $alya_fmt_struct_open\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_struct_loop:\n");
    out.push_str("    cmp 4(%edi), %ebx\n");
    out.push_str("    jge .L_x86_struct_close\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_struct_print_f\n");
    out.push_str("    push $alya_fmt_struct_comma\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_struct_print_f:\n");
    out.push_str("    push 4(%esi, %ebx, 4)\n");
    out.push_str("    push 8(%edi, %ebx, 4)\n");
    out.push_str("    push $alya_fmt_struct_field\n");
    out.push_str("    call printf\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_struct_loop\n");
    out.push_str(".L_x86_struct_close:\n");
    out.push_str("    push $alya_fmt_struct_close\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_struct_exit:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

}
