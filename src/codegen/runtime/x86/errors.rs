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

    // Per-thread catch state (alya-lang/alya#65). Heap block layout
    // (zeroed via calloc): idx@0, handler[128]@8, sp[128]@1032,
    // bp[128]@2056, err_msg@3080, err_str@3088 (3096 bytes). The
    // key/FLS slot is created once via pthread_once / InitOnceExecuteOnce;
    // each thread's block is freed by the key destructor / FLS callback.
    // Field widths stay 8 bytes like the x64 layout (32-bit ops touch
    // the low halves; indices never exceed 128).
    out.push_str(".global alya_catch_init\n");
    out.push_str("alya_catch_init:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        // stdcall-decorated kernel32 imports (mingw has no bare aliases
        // for these Vista+ APIs on x86). stdcall callee pops its own
        // args: the caller must NOT adjust esp afterwards.
        out.push_str("    push $alya_catch_free\n");
        out.push_str("    call _FlsAlloc@4\n");
        out.push_str("    mov %eax, alya_catch_fls\n");
        out.push_str("    mov $1, %eax\n");
    } else {
        out.push_str("    push $alya_catch_free\n");
        out.push_str("    push $alya_catch_key\n");
        out.push_str("    call pthread_key_create\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    xor %eax, %eax\n");
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    // NOTE: __stdcall callback (InitOnceExecuteOnce): callee pops its
    // 12 argument bytes. The pthread path is cdecl (plain ret).
    if is_win {
        out.push_str("    ret $12\n\n");
    } else {
        out.push_str("    ret\n\n");
    }

    // alya_catch_free: per-thread block destructor (never sees null).
    out.push_str(".global alya_catch_free\n");
    out.push_str("alya_catch_free:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push 8(%ebp)\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    // NOTE: __stdcall FLS callback (one 4-byte argument); the pthread
    // path is cdecl (plain ret).
    if is_win {
        out.push_str("    ret $4\n\n");
    } else {
        out.push_str("    ret\n\n");
    }

    // alya_catch_block: current thread's catch-state block (allocates
    // on first use). Exits the process on allocation failure.
    out.push_str(".global alya_catch_block\n");
    out.push_str("alya_catch_block:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    if is_win {
        out.push_str("    push $0\n");
        out.push_str("    push $0\n");
        out.push_str("    push $alya_catch_init\n");
        out.push_str("    push $alya_catch_once\n");
        out.push_str("    call _InitOnceExecuteOnce@16\n");
        out.push_str("    mov alya_catch_fls, %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call _FlsGetValue@4\n");
    } else {
        out.push_str("    push $alya_catch_init\n");
        out.push_str("    push $alya_catch_once\n");
        out.push_str("    call pthread_once\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    mov alya_catch_key, %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call pthread_getspecific\n");
        out.push_str("    add $4, %esp\n");
    }
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_catch_block_ret\n");
    out.push_str("    push $3096\n");
    out.push_str("    push $1\n");
    out.push_str("    call calloc\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_catch_block_oom\n");
    out.push_str("    mov %eax, %ebx\n");
    if is_win {
        out.push_str("    push %ebx\n");
        out.push_str("    mov alya_catch_fls, %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call _FlsSetValue@8\n");
    } else {
        out.push_str("    push %ebx\n");
        out.push_str("    mov alya_catch_key, %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    call pthread_setspecific\n");
        out.push_str("    add $8, %esp\n");
    }
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    jmp .L_x86_catch_block_ret\n");
    out.push_str(".L_x86_catch_block_oom:\n");
    out.push_str("    push $1\n");
    out.push_str("    call exit\n");
    out.push_str(".L_x86_catch_block_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_try_begin(handler, sp, bp): push one catch frame.
    out.push_str(".global alya_try_begin\n");
    out.push_str("alya_try_begin:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    call alya_catch_block\n");
    out.push_str("    mov %eax, %esi\n");
    out.push_str("    mov (%esi), %ecx\n");
    out.push_str("    cmp $128, %ecx\n");
    out.push_str("    jae .L_x86_try_begin_oom\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    mov %ebx, 8(%esi, %ecx, 8)\n");
    out.push_str("    mov 12(%ebp), %ebx\n");
    out.push_str("    mov %ebx, 1032(%esi, %ecx, 8)\n");
    out.push_str("    mov 16(%ebp), %ebx\n");
    out.push_str("    mov %ebx, 2056(%esi, %ecx, 8)\n");
    out.push_str("    incl (%esi)\n");
    out.push_str("    jmp .L_x86_try_begin_ret\n");
    out.push_str(".L_x86_try_begin_oom:\n");
    out.push_str("    push $1\n");
    out.push_str("    call exit\n");
    out.push_str(".L_x86_try_begin_ret:\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_try_end: pop one catch frame.
    out.push_str(".global alya_try_end\n");
    out.push_str("alya_try_end:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    call alya_catch_block\n");
    out.push_str("    decl (%eax)\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // alya_catch_msg: in-flight error value for `catch` bindings.
    out.push_str(".global alya_catch_msg\n");
    out.push_str("alya_catch_msg:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    call alya_catch_block\n");
    out.push_str("    mov 3080(%eax), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_throw
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    call alya_catch_block\n");
    out.push_str("    mov %eax, %esi\n");
    out.push_str("    mov %ebx, 3080(%esi)\n");
    out.push_str("    mov 12(%ebp), %eax\n");
    out.push_str("    mov %eax, 3088(%esi)\n");
    out.push_str("    mov (%esi), %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_fatal_throw\n");
    out.push_str("    dec %ecx\n");
    out.push_str("    mov %ecx, (%esi)\n");
    out.push_str("    mov 8(%esi, %ecx, 8), %eax\n");
    out.push_str("    mov 1032(%esi, %ecx, 8), %esp\n");
    out.push_str("    mov 2056(%esi, %ecx, 8), %ebp\n");
    out.push_str("    jmp *%eax\n");
    out.push_str(".L_x86_fatal_throw:\n");
    out.push_str("    and $-16, %esp\n");
    // Prefer the extracted struct-message text when throw codegen set
    // it; the raw value may be a struct pointer, not a string.
    out.push_str("    mov 3088(%esi), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_fatal_have_msg\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str(".L_x86_fatal_have_msg:\n");
    out.push_str("    push %eax\n");
    out.push_str("    push $alya_fmt_runtime_err\n");
    out.push_str("    call printf\n");
    out.push_str("    push $0\n");
    out.push_str("    call fflush\n");
    out.push_str("    push $1\n");
    out.push_str("    call exit\n\n");

    // fn_rethrow: rethrows the in-flight value with its stored message.
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    call alya_catch_block\n");
    out.push_str("    mov 3088(%eax), %edx\n");
    out.push_str("    mov 3080(%eax), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_rethrow_has_msg\n");
    out.push_str("    mov $alya_str_unhandled_err, %eax\n");
    out.push_str(".L_x86_rethrow_has_msg:\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    out.push_str("    push $0\n");
    out.push_str("    push $alya_str_div_zero\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    out.push_str("    push $0\n");
    out.push_str("    push $alya_str_bounds\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    out.push_str("    push $0\n");
    out.push_str("    push $alya_str_null_unwrap\n");
    out.push_str("    call fn_throw\n\n");

    // alya_error_mixed_float (dynamic mixed int/float arithmetic,
    // alya-lang/alya#39; see the x64 note above).
    out.push_str("alya_error_mixed_float:\n");
    out.push_str("    push $0\n");
    out.push_str("    push $alya_str_mixed_float\n");
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
