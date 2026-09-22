use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_exit
    out.push_str("fn_exit:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    push %rcx\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fflush\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    pop %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call exit\n");
    } else {
        out.push_str("    push %rdi\n");
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str(&format!("    call {}fflush\n", p));
        out.push_str("    add $8, %rsp\n");
        out.push_str("    pop %rdi\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str(&format!("    call {}exit\n", p));
    }

    // fn_throw
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    if is_win {
        out.push_str("    mov %rcx, alya_err_msg(%rip)\n");
    } else {
        out.push_str("    mov %rdi, alya_err_msg(%rip)\n");
    }
    out.push_str("    mov alya_catch_idx(%rip), %r8\n");
    out.push_str("    test %r8, %r8\n");
    out.push_str("    jz .L_x64_fatal_throw\n");
    out.push_str("    dec %r8\n");
    out.push_str("    mov %r8, alya_catch_idx(%rip)\n");
    out.push_str("    lea alya_catch_stack_sp(%rip), %r9\n");
    out.push_str("    mov (%r9, %r8, 8), %rsp\n");
    out.push_str("    lea alya_catch_stack_bp(%rip), %r9\n");
    out.push_str("    mov (%r9, %r8, 8), %rbp\n");
    out.push_str("    lea alya_catch_stack_handler(%rip), %r9\n");
    out.push_str("    mov (%r9, %r8, 8), %r10\n");
    out.push_str("    jmp *%r10\n");
    out.push_str(".L_x64_fatal_throw:\n");
    out.push_str("    and $-16, %rsp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rcx, %rdx\n");
        out.push_str("    lea alya_fmt_runtime_err(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call fflush\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    call exit\n\n");
    } else {
        out.push_str("    mov %rdi, %rsi\n");
        out.push_str("    lea alya_fmt_runtime_err(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str(&format!("    call {}fflush\n", p));
        out.push_str("    mov $1, %rdi\n");
        out.push_str(&format!("    call {}exit\n\n", p));
    }

    // fn_rethrow
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    out.push_str("    mov alya_err_msg(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_rethrow_has_msg\n");
    out.push_str("    lea alya_str_unhandled_err(%rip), %rax\n");
    out.push_str(".L_x64_rethrow_has_msg:\n");
    if is_win {
        out.push_str("    mov %rax, %rcx\n");
    } else {
        out.push_str("    mov %rax, %rdi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    if is_win {
        out.push_str("    lea alya_str_div_zero(%rip), %rcx\n");
    } else {
        out.push_str("    lea alya_str_div_zero(%rip), %rdi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    if is_win {
        out.push_str("    lea alya_str_bounds(%rip), %rcx\n");
    } else {
        out.push_str("    lea alya_str_bounds(%rip), %rdi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    if is_win {
        out.push_str("    lea alya_str_null_unwrap(%rip), %rcx\n");
    } else {
        out.push_str("    lea alya_str_null_unwrap(%rip), %rdi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // fn_sleep
    out.push_str(".global fn_sleep\n");
    out.push_str("fn_sleep:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    call Sleep\n");
    } else {
        out.push_str("    imul $1000, %rdi, %rdi\n");
        out.push_str(&format!("    call {}usleep\n", p));
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_system_exec
    out.push_str(".global fn_system_exec\n");
    out.push_str("fn_system_exec:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    test %rcx, %rcx\n");
        out.push_str("    jz .L_x64_sysexec_empty\n");
        out.push_str("    call system\n");
    } else {
        out.push_str("    test %rdi, %rdi\n");
        out.push_str("    jz .L_x64_sysexec_empty\n");
        out.push_str(&format!("    call {}system\n", p));
    }
    out.push_str("    jmp .L_x64_sysexec_ret\n");
    out.push_str(".L_x64_sysexec_empty:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_sysexec_ret:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_get_pid
    out.push_str(".global fn___native_get_pid\n");
    out.push_str("fn___native_get_pid:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call GetCurrentProcessId\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str(&format!("    call {}getpid\n", p));
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_get_cwd
    out.push_str(".global fn___native_get_cwd\n");
    out.push_str("fn___native_get_cwd:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $1072, %rsp\n");
    if is_win {
        out.push_str("    mov $1024, %rcx\n");
        out.push_str("    lea 32(%rsp), %rdx\n");
        out.push_str("    call GetCurrentDirectoryA\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_gcwd_empty\n");
        out.push_str("    lea 32(%rsp), %rcx\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    jmp .L_x64_gcwd_done\n");
    } else {
        out.push_str("    lea 32(%rsp), %rdi\n");
        out.push_str("    mov $1024, %rsi\n");
        out.push_str(&format!("    call {}getcwd\n", p));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_gcwd_empty\n");
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call fn_str_clone\n");
        out.push_str("    jmp .L_x64_gcwd_done\n");
    }
    out.push_str(".L_x64_gcwd_empty:\n");
    out.push_str("    lea alya_str_empty(%rip), %rax\n");
    out.push_str(".L_x64_gcwd_done:\n");
    out.push_str("    add $1072, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_set_cwd
    out.push_str(".global fn___native_set_cwd\n");
    out.push_str("fn___native_set_cwd:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    test %rcx, %rcx\n");
        out.push_str("    jz .L_x64_scwd_fail\n");
        out.push_str("    call SetCurrentDirectoryA\n");
        out.push_str("    test %rax, %rax\n");
        out.push_str("    setne %al\n");
        out.push_str("    movzbq %al, %rax\n");
        out.push_str("    jmp .L_x64_scwd_done\n");
        out.push_str(".L_x64_scwd_fail:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_scwd_done:\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    test %rdi, %rdi\n");
        out.push_str("    jz .L_x64_scwd_fail\n");
        out.push_str(&format!("    call {}chdir\n", p));
        out.push_str("    test %rax, %rax\n");
        out.push_str("    sete %al\n");
        out.push_str("    movzbq %al, %rax\n");
        out.push_str("    jmp .L_x64_scwd_done\n");
        out.push_str(".L_x64_scwd_fail:\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(".L_x64_scwd_done:\n");
        out.push_str("    add $8, %rsp\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
