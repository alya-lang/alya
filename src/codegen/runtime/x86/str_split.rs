use crate::codegen::target::OperatingSystem;
use super::{emit_str_buf_load, emit_str_buf_store};

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_contains
    out.push_str("fn_contains:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    cmpb $0, (%edi)\n");
    out.push_str("    je .L_x86_contains_match\n");
    out.push_str(".L_x86_contains_outer:\n");
    out.push_str("    cmpb $0, (%esi)\n");
    out.push_str("    je .L_x86_contains_nomatch\n");
    out.push_str("    mov %esi, %eax\n");
    out.push_str("    mov %edi, %edx\n");
    out.push_str(".L_x86_contains_inner:\n");
    out.push_str("    movb (%edx), %cl\n");
    out.push_str("    test %cl, %cl\n");
    out.push_str("    jz .L_x86_contains_match\n");
    out.push_str("    movb (%eax), %bl\n");
    out.push_str("    cmp %bl, %cl\n");
    out.push_str("    jne .L_x86_contains_next\n");
    out.push_str("    inc %eax\n");
    out.push_str("    inc %edx\n");
    out.push_str("    jmp .L_x86_contains_inner\n");
    out.push_str(".L_x86_contains_next:\n");
    out.push_str("    inc %esi\n");
    out.push_str("    jmp .L_x86_contains_outer\n");
    out.push_str(".L_x86_contains_match:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_contains_end\n");
    out.push_str(".L_x86_contains_nomatch:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_contains_end:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_join
    out.push_str(".global fn_join\n");
    out.push_str("fn_join:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov %eax, -8(%ebp)\n"); // arr
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov %eax, -12(%ebp)\n"); // delim
    out.push_str("    mov -8(%ebp), %esi\n");
    out.push_str("    test %esi, %esi\n");
    out.push_str("    jz .L_x86_join_empty\n");
    out.push_str("    mov (%esi), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_join_empty\n");
    emit_str_buf_load(out, "%ecx", "%ebx", os);
    out.push_str("    cmp $1000000, %ebx\n");
    out.push_str("    jb .L_x86_join_buf_ok\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_join_buf_ok:\n");
    out.push_str("    lea (%ecx, %ebx), %edi\n");
    out.push_str("    mov %edi, -4(%ebp)\n"); // start_ptr
    out.push_str("    movl $0, -16(%ebp)\n"); // i = 0
    out.push_str(".L_x86_join_loop:\n");
    out.push_str("    mov -8(%ebp), %esi\n");
    out.push_str("    mov -16(%ebp), %eax\n");
    out.push_str("    cmp (%esi), %eax\n");
    out.push_str("    jge .L_x86_join_finish\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_join_copy_elem\n");
    out.push_str("    mov -12(%ebp), %esi\n"); // copy delim
    out.push_str(".L_x86_join_copy_delim:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_join_copy_elem\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_join_copy_delim\n");
    out.push_str(".L_x86_join_copy_elem:\n");
    out.push_str("    mov -8(%ebp), %esi\n");
    out.push_str("    mov 8(%esi), %esi\n");
    out.push_str("    mov -16(%ebp), %eax\n");
    out.push_str("    mov (%esi, %eax, 4), %esi\n");
    out.push_str(".L_x86_join_copy_elem_loop:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_join_elem_done\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_join_copy_elem_loop\n");
    out.push_str(".L_x86_join_elem_done:\n");
    out.push_str("    incl -16(%ebp)\n");
    out.push_str("    jmp .L_x86_join_loop\n");
    out.push_str(".L_x86_join_finish:\n");
    out.push_str("    movb $0, (%edi)\n");
    out.push_str("    inc %edi\n");
    out.push_str("    sub %ecx, %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%edx", os);
    out.push_str("    mov -4(%ebp), %eax\n");
    out.push_str("    jmp .L_x86_join_ret\n");
    out.push_str(".L_x86_join_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_join_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_split
    out.push_str(".global fn_split\n");
    out.push_str("fn_split:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $32, %esp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov %eax, -4(%ebp)\n"); // str
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov %eax, -8(%ebp)\n"); // delim
    out.push_str("    push $0\n");
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, -12(%ebp)\n"); // arr
    out.push_str("    mov -8(%ebp), %esi\n");
    out.push_str("    xor %ecx, %ecx\n");
    out.push_str(".L_x86_split_dlen_loop:\n");
    out.push_str("    cmpb $0, (%esi, %ecx)\n");
    out.push_str("    je .L_x86_split_dlen_done\n");
    out.push_str("    inc %ecx\n");
    out.push_str("    jmp .L_x86_split_dlen_loop\n");
    out.push_str(".L_x86_split_dlen_done:\n");
    out.push_str("    mov %ecx, -16(%ebp)\n"); // delim_len
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jnz .L_x86_split_non_empty_delim\n");
    out.push_str("    mov -4(%ebp), %esi\n"); // empty delim
    out.push_str(".L_x86_split_empty_loop:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    test %al, %al\n");
    out.push_str("    jz .L_x86_split_ret\n");
    emit_str_buf_load(out, "%ecx", "%ebx", os);
    out.push_str("    cmp $1000000, %ebx\n");
    out.push_str("    jb .L_x86_se1\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_se1:\n");
    out.push_str("    lea (%ecx, %ebx), %edi\n");
    out.push_str("    mov %edi, %edx\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    movb $0, 1(%edi)\n");
    out.push_str("    add $2, %edi\n");
    out.push_str("    sub %ecx, %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%ecx", os);
    out.push_str("    push $3\n"); // kind = string
    out.push_str("    push $0\n"); // hi
    out.push_str("    push %edx\n");
    out.push_str("    push -12(%ebp)\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    inc %esi\n");
    out.push_str("    jmp .L_x86_split_empty_loop\n");
    out.push_str(".L_x86_split_non_empty_delim:\n");
    out.push_str("    mov -4(%ebp), %eax\n");
    out.push_str("    mov %eax, -20(%ebp)\n"); // curr
    out.push_str("    mov %eax, -24(%ebp)\n"); // token_start
    out.push_str(".L_x86_split_main_loop:\n");
    out.push_str("    mov -20(%ebp), %esi\n");
    out.push_str("    cmpb $0, (%esi)\n");
    out.push_str("    je .L_x86_split_emit_final\n");
    out.push_str("    mov -16(%ebp), %ecx\n"); // delim_len
    out.push_str("    mov -8(%ebp), %edx\n"); // delim
    out.push_str("    xor %ebx, %ebx\n"); // k = 0
    out.push_str(".L_x86_split_cmp_loop:\n");
    out.push_str("    cmp %ecx, %ebx\n");
    out.push_str("    jge .L_x86_split_matched\n");
    out.push_str("    movb (%edx, %ebx), %al\n");
    out.push_str("    cmpb %al, (%esi, %ebx)\n");
    out.push_str("    jne .L_x86_split_cmp_fail\n");
    out.push_str("    inc %ebx\n");
    out.push_str("    jmp .L_x86_split_cmp_loop\n");
    out.push_str(".L_x86_split_cmp_fail:\n");
    out.push_str("    incl -20(%ebp)\n");
    out.push_str("    jmp .L_x86_split_main_loop\n");
    out.push_str(".L_x86_split_matched:\n");
    emit_str_buf_load(out, "%ecx", "%ebx", os);
    out.push_str("    cmp $1000000, %ebx\n");
    out.push_str("    jb .L_x86_se2\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_se2:\n");
    out.push_str("    lea (%ecx, %ebx), %edi\n");
    out.push_str("    mov %edi, -28(%ebp)\n");
    out.push_str("    mov -24(%ebp), %esi\n");
    out.push_str("    mov -20(%ebp), %edx\n");
    out.push_str(".L_x86_split_tok_copy:\n");
    out.push_str("    cmp %edx, %esi\n");
    out.push_str("    jge .L_x86_split_tok_done\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_split_tok_copy\n");
    out.push_str(".L_x86_split_tok_done:\n");
    out.push_str("    movb $0, (%edi)\n");
    out.push_str("    inc %edi\n");
    out.push_str("    sub %ecx, %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%edx", os);
    out.push_str("    push $3\n"); // kind = string
    out.push_str("    push $0\n"); // hi
    out.push_str("    push -28(%ebp)\n");
    out.push_str("    push -12(%ebp)\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    mov -16(%ebp), %eax\n");
    out.push_str("    add %eax, -20(%ebp)\n");
    out.push_str("    mov -20(%ebp), %eax\n");
    out.push_str("    mov %eax, -24(%ebp)\n");
    out.push_str("    jmp .L_x86_split_main_loop\n");
    out.push_str(".L_x86_split_emit_final:\n");
    emit_str_buf_load(out, "%ecx", "%ebx", os);
    out.push_str("    cmp $1000000, %ebx\n");
    out.push_str("    jb .L_x86_se3\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_se3:\n");
    out.push_str("    lea (%ecx, %ebx), %edi\n");
    out.push_str("    mov %edi, -28(%ebp)\n");
    out.push_str("    mov -24(%ebp), %esi\n");
    out.push_str("    mov -20(%ebp), %edx\n");
    out.push_str(".L_x86_split_final_copy:\n");
    out.push_str("    cmp %edx, %esi\n");
    out.push_str("    jge .L_x86_split_final_done\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_split_final_copy\n");
    out.push_str(".L_x86_split_final_done:\n");
    out.push_str("    movb $0, (%edi)\n");
    out.push_str("    inc %edi\n");
    out.push_str("    sub %ecx, %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%edx", os);
    out.push_str("    push $3\n"); // kind = string
    out.push_str("    push $0\n"); // hi
    out.push_str("    push -28(%ebp)\n");
    out.push_str("    push -12(%ebp)\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
    out.push_str(".L_x86_split_ret:\n");
    out.push_str("    mov -12(%ebp), %eax\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

}
