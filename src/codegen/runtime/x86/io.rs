use crate::codegen::target::OperatingSystem;
use super::{emit_str_buf_load, emit_str_buf_store};

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_ask
    out.push_str("fn_ask:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_ask_read\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_prompt\n");
    out.push_str("    call printf\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    push $0\n");
    out.push_str("    call fflush\n");
    out.push_str("    add $4, %esp\n");
    out.push_str(".L_x86_ask_read:\n");
    emit_str_buf_load(out, "%ecx", "%ebx", os);
    out.push_str("    cmp $1000000, %ebx\n");
    out.push_str("    jb .L_x86_ask_buf_ok\n");
    out.push_str("    xor %ebx, %ebx\n");
    out.push_str(".L_x86_ask_buf_ok:\n");
    out.push_str("    lea (%ecx, %ebx), %esi\n");
    out.push_str("    mov %esi, %edi\n");
    // Keep the buffer base in callee-saved %ebx: `getchar` below may
    // clobber caller-saved %ecx, which then corrupts the size math and
    // the stored index (the next read would restart at buf+0 and
    // overwrite this string).
    out.push_str("    mov %ecx, %ebx\n");
    out.push_str(".L_x86_ask_loop:\n");
    out.push_str("    call getchar\n");
    out.push_str("    cmp $-1, %eax\n");
    out.push_str("    je .L_x86_ask_done\n");
    out.push_str("    cmp $10, %eax\n");
    out.push_str("    je .L_x86_ask_done\n");
    out.push_str("    cmp $13, %eax\n");
    out.push_str("    je .L_x86_ask_loop\n");
    out.push_str("    movb %al, (%edi)\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_ask_loop\n");
    out.push_str(".L_x86_ask_done:\n");
    out.push_str("    movb $0, (%edi)\n");
    out.push_str("    inc %edi\n");
    out.push_str("    sub %ebx, %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%edx", os);
    out.push_str("    mov %esi, %eax\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_get_env
    out.push_str(".global fn_get_env\n");
    out.push_str("fn_get_env:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_getenv_empty\n");
    out.push_str("    push %eax\n");
    out.push_str("    call getenv\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_getenv_empty\n");
    out.push_str("    mov %eax, %esi\n");
    emit_str_buf_load(out, "%ebx", "%edi", os);
    out.push_str("    cmp $1000000, %edi\n");
    out.push_str("    jb .L_x86_getenv_buf_ok\n");
    out.push_str("    xor %edi, %edi\n");
    out.push_str(".L_x86_getenv_buf_ok:\n");
    out.push_str("    lea (%ebx, %edi), %edx\n");
    out.push_str(".L_x86_getenv_copy:\n");
    out.push_str("    movb (%esi), %al\n");
    out.push_str("    movb %al, (%ebx, %edi)\n");
    out.push_str("    testb %al, %al\n");
    out.push_str("    jz .L_x86_getenv_done\n");
    out.push_str("    inc %esi\n");
    out.push_str("    inc %edi\n");
    out.push_str("    jmp .L_x86_getenv_copy\n");
    out.push_str(".L_x86_getenv_done:\n");
    out.push_str("    inc %edi\n");
    out.push_str("    add $3, %edi\n");
    out.push_str("    and $-4, %edi\n");
    emit_str_buf_store(out, "%edi", "%ecx", os);
    out.push_str("    mov %edx, %eax\n");
    out.push_str("    jmp .L_x86_getenv_ret\n");
    out.push_str(".L_x86_getenv_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_getenv_ret:\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_target_os
    out.push_str(".global fn_target_os\n");
    out.push_str("fn_target_os:\n");
    out.push_str("    mov $alya_str_target_os, %eax\n");
    out.push_str("    ret\n\n");

    // fn_target_arch
    out.push_str(".global fn_target_arch\n");
    out.push_str("fn_target_arch:\n");
    out.push_str("    mov $alya_str_target_arch, %eax\n");
    out.push_str("    ret\n\n");

    // fn_set_console_output_cp
    out.push_str(".global fn_set_console_output_cp\n");
    out.push_str("fn_set_console_output_cp:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");

    // fn_set_console_input_cp
    out.push_str(".global fn_set_console_input_cp\n");
    out.push_str("fn_set_console_input_cp:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");

    // fn_get_console_output_cp
    out.push_str(".global fn_get_console_output_cp\n");
    out.push_str("fn_get_console_output_cp:\n");
    out.push_str("    mov $65001, %eax\n");
    out.push_str("    ret\n\n");

    // fn_get_console_input_cp
    out.push_str(".global fn_get_console_input_cp\n");
    out.push_str("fn_get_console_input_cp:\n");
    out.push_str("    mov $65001, %eax\n");
    out.push_str("    ret\n\n");

    // fn_enable_virtual_terminal
    out.push_str(".global fn_enable_virtual_terminal\n");
    out.push_str("fn_enable_virtual_terminal:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");

    // fn_set_console_title
    out.push_str(".global fn_set_console_title\n");
    out.push_str("fn_set_console_title:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");

    // fn_beep_console
    out.push_str(".global fn_beep_console\n");
    out.push_str("fn_beep_console:\n");
    out.push_str("    push $alya_str_console_bell\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    push $0\n");
    out.push_str("    call fflush\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");

    // fn_clear_console
    out.push_str(".global fn_clear_console\n");
    out.push_str("fn_clear_console:\n");
    out.push_str("    push $alya_str_console_clear\n");
    out.push_str("    call printf\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    push $0\n");
    out.push_str("    call fflush\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    ret\n\n");
}

