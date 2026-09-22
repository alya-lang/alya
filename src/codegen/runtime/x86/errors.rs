use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_exit
    out.push_str("fn_exit:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call exit\n\n");

    // fn_throw
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    out.push_str("    mov 4(%esp), %eax\n");
    out.push_str("    mov %eax, alya_err_msg\n");
    out.push_str("    mov alya_catch_idx, %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_fatal_throw\n");
    out.push_str("    dec %ecx\n");
    out.push_str("    mov %ecx, alya_catch_idx\n");
    out.push_str("    mov $alya_catch_stack_sp, %edx\n");
    out.push_str("    mov (%edx, %ecx, 4), %esp\n");
    out.push_str("    mov $alya_catch_stack_bp, %edx\n");
    out.push_str("    mov (%edx, %ecx, 4), %ebp\n");
    out.push_str("    mov $alya_catch_stack_handler, %edx\n");
    out.push_str("    mov (%edx, %ecx, 4), %eax\n");
    out.push_str("    jmp *%eax\n");
    out.push_str(".L_x86_fatal_throw:\n");
    out.push_str("    and $-16, %esp\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_runtime_err\n");
    out.push_str("    call printf\n");
    out.push_str("    push $0\n");
    out.push_str("    call fflush\n");
    out.push_str("    push $1\n");
    out.push_str("    call exit\n\n");

    // fn_rethrow
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    out.push_str("    mov alya_err_msg, %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_rethrow_has_msg\n");
    out.push_str("    mov $alya_str_unhandled_err, %eax\n");
    out.push_str(".L_x86_rethrow_has_msg:\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    out.push_str("    push $alya_str_div_zero\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    out.push_str("    push $alya_str_bounds\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    out.push_str("    push $alya_str_null_unwrap\n");
    out.push_str("    call fn_throw\n\n");

    // fn_sleep
    out.push_str(".global fn_sleep\n");
    out.push_str("fn_sleep:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    push %eax\n");
        out.push_str("    call Sleep\n");
    } else {
        out.push_str("    imul $1000, %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call usleep\n");
        out.push_str("    add $4, %esp\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_system_exec
    out.push_str(".global fn_system_exec\n");
    out.push_str("fn_system_exec:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_sysexec_empty\n");
    out.push_str("    push %eax\n");
    out.push_str("    call system\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    jmp .L_x86_sysexec_ret\n");
    out.push_str(".L_x86_sysexec_empty:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_sysexec_ret:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_get_pid
    out.push_str(".global fn___native_get_pid\n");
    out.push_str("fn___native_get_pid:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    call GetCurrentProcessId\n");
    } else {
        out.push_str("    call getpid\n");
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_get_cwd
    out.push_str(".global fn___native_get_cwd\n");
    out.push_str("fn___native_get_cwd:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $1028, %esp\n");
    out.push_str("    lea -1024(%ebp), %eax\n");
    if is_win {
        out.push_str("    push %eax\n");
        out.push_str("    push $1024\n");
        out.push_str("    call GetCurrentDirectoryA\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    jz .L_x86_gcwd_empty\n");
        out.push_str("    lea -1024(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    add $4, %esp\n");
        out.push_str("    jmp .L_x86_gcwd_done\n");
    } else {
        out.push_str("    push $1024\n");
        out.push_str("    push %eax\n");
        out.push_str("    call getcwd\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    jz .L_x86_gcwd_empty\n");
        out.push_str("    push %eax\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    add $4, %esp\n");
        out.push_str("    jmp .L_x86_gcwd_done\n");
    }
    out.push_str(".L_x86_gcwd_empty:\n");
    out.push_str("    mov $alya_str_empty, %eax\n");
    out.push_str(".L_x86_gcwd_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_set_cwd
    out.push_str(".global fn___native_set_cwd\n");
    out.push_str("fn___native_set_cwd:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_scwd_fail\n");
    if is_win {
        out.push_str("    push %eax\n");
        out.push_str("    call SetCurrentDirectoryA\n");
        out.push_str("    add $4, %esp\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    setne %al\n");
        out.push_str("    movzbl %al, %eax\n");
        out.push_str("    jmp .L_x86_scwd_done\n");
    } else {
        out.push_str("    push %eax\n");
        out.push_str("    call chdir\n");
        out.push_str("    add $4, %esp\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    sete %al\n");
        out.push_str("    movzbl %al, %eax\n");
        out.push_str("    jmp .L_x86_scwd_done\n");
    }
    out.push_str(".L_x86_scwd_fail:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_scwd_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
