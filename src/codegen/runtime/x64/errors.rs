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

    // Per-thread catch state (alya-lang/alya#65). Heap block layout
    // (zeroed via calloc): idx@0, handler[128]@8, sp[128]@1032,
    // bp[128]@2056, err_msg@3080, err_str@3088 (3096 bytes). The
    // key/FLS slot is created once via pthread_once / InitOnceExecuteOnce;
    // each thread's block is freed by the key destructor / FLS callback.
    out.push_str(".global alya_catch_init\n");
    out.push_str("alya_catch_init:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        // FlsAlloc takes only the callback; the index comes back in eax.
        out.push_str("    lea alya_catch_free(%rip), %rcx\n");
        out.push_str("    call FlsAlloc\n");
        out.push_str("    mov %eax, alya_catch_fls(%rip)\n");
        out.push_str("    mov $1, %eax\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    lea alya_catch_key(%rip), %rdi\n");
        out.push_str("    lea alya_catch_free(%rip), %rsi\n");
        out.push_str(&format!("    call {}pthread_key_create\n", p));
        out.push_str("    xor %eax, %eax\n");
    }
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_catch_free: per-thread block destructor (never sees null).
    out.push_str(".global alya_catch_free\n");
    out.push_str("alya_catch_free:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call free\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_catch_block: current thread's catch-state block (allocates
    // on first use). Exits the process on allocation failure.
    out.push_str(".global alya_catch_block\n");
    out.push_str("alya_catch_block:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    lea alya_catch_once(%rip), %rcx\n");
        out.push_str("    lea alya_catch_init(%rip), %rdx\n");
        out.push_str("    xor %r8d, %r8d\n");
        out.push_str("    xor %r9d, %r9d\n");
        out.push_str("    call InitOnceExecuteOnce\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov alya_catch_fls(%rip), %ecx\n");
        out.push_str("    call FlsGetValue\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    lea alya_catch_once(%rip), %rdi\n");
        out.push_str("    lea alya_catch_init(%rip), %rsi\n");
        out.push_str(&format!("    call {}pthread_once\n", p));
        out.push_str("    mov alya_catch_key(%rip), %edi\n");
        out.push_str(&format!("    call {}pthread_getspecific\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_catch_block_ret\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov $1, %ecx\n");
        out.push_str("    mov $3096, %edx\n");
        out.push_str("    call calloc\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov $1, %rdi\n");
        out.push_str("    mov $3096, %rsi\n");
        out.push_str(&format!("    call {}calloc\n", p));
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_catch_block_oom\n");
    out.push_str("    mov %rax, %rbx\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov alya_catch_fls(%rip), %ecx\n");
        out.push_str("    mov %rbx, %rdx\n");
        out.push_str("    call FlsSetValue\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov alya_catch_key(%rip), %edi\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str(&format!("    call {}pthread_setspecific\n", p));
    }
    out.push_str("    mov %rbx, %rax\n");
    out.push_str("    jmp .L_x64_catch_block_ret\n");
    out.push_str(".L_x64_catch_block_oom:\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    call exit\n");
    } else {
        out.push_str("    mov $1, %rdi\n");
        out.push_str(&format!("    call {}exit\n", p));
    }
    out.push_str(".L_x64_catch_block_ret:\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_try_begin(handler, sp, bp): push one catch frame.
    out.push_str(".global alya_try_begin\n");
    out.push_str("alya_try_begin:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    mov %rdx, %r12\n");
        out.push_str("    mov %r8, %r13\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_catch_block\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r12\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    call alya_catch_block\n");
    }
    out.push_str("    mov (%rax), %r14\n");
    out.push_str("    cmp $128, %r14\n");
    out.push_str("    jae .L_x64_try_begin_oom\n");
    out.push_str("    mov %rbx, 8(%rax, %r14, 8)\n");
    out.push_str("    mov %r12, 1032(%rax, %r14, 8)\n");
    out.push_str("    mov %r13, 2056(%rax, %r14, 8)\n");
    out.push_str("    incq (%rax)\n");
    out.push_str("    jmp .L_x64_try_begin_ret\n");
    out.push_str(".L_x64_try_begin_oom:\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    call exit\n");
    } else {
        out.push_str("    mov $1, %rdi\n");
        out.push_str(&format!("    call {}exit\n", p));
    }
    out.push_str(".L_x64_try_begin_ret:\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_try_end: pop one catch frame.
    out.push_str(".global alya_try_end\n");
    out.push_str("alya_try_end:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_catch_block\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    call alya_catch_block\n");
    }
    out.push_str("    decq (%rax)\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // alya_catch_msg: in-flight error value for `catch` bindings.
    out.push_str(".global alya_catch_msg\n");
    out.push_str("alya_catch_msg:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_catch_block\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    call alya_catch_block\n");
    }
    out.push_str("    mov 3080(%rax), %rax\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_throw
    // fn_throw(value, msg): records both in the thread's block.
    // msg carries struct `message` text (or null); the fatal printer
    // prefers it over the raw value (which may be a struct pointer).
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    // Five pushes keep rsp 16-aligned for the block call below.
    // (No epilogue pops: this function never returns normally —
    // handler jumps restore rsp/rbp, the fatal path exits.)
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    mov %rdx, %r13\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_catch_block\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    mov %rsi, %r13\n");
        out.push_str("    call alya_catch_block\n");
    }
    out.push_str("    mov %rax, %r12\n");
    out.push_str("    mov %rbx, 3080(%r12)\n");
    out.push_str("    mov %r13, 3088(%r12)\n");
    out.push_str("    mov (%r12), %r8\n");
    out.push_str("    test %r8, %r8\n");
    out.push_str("    jz .L_x64_fatal_throw\n");
    out.push_str("    dec %r8\n");
    out.push_str("    mov %r8, (%r12)\n");
    out.push_str("    mov 8(%r12, %r8, 8), %r10\n");
    out.push_str("    mov 1032(%r12, %r8, 8), %rsp\n");
    out.push_str("    mov 2056(%r12, %r8, 8), %rbp\n");
    out.push_str("    jmp *%r10\n");
    out.push_str(".L_x64_fatal_throw:\n");
    out.push_str("    and $-16, %rsp\n");
    // Prefer the struct `message` text carried as fn_throw's second
    // argument (block+3088); the raw value may be a struct pointer,
    // not a printable string.
    if is_win {
        out.push_str("    mov 3088(%r12), %rdx\n");
        out.push_str("    test %rdx, %rdx\n");
        out.push_str("    jnz .L_x64_fatal_have_msg\n");
        out.push_str("    mov %rbx, %rdx\n");
        out.push_str(".L_x64_fatal_have_msg:\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    lea alya_fmt_runtime_err(%rip), %rcx\n");
        out.push_str("    call printf\n");
        out.push_str("    xor %rcx, %rcx\n");
        out.push_str("    call fflush\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    call exit\n\n");
    } else {
        out.push_str("    mov 3088(%r12), %rsi\n");
        out.push_str("    test %rsi, %rsi\n");
        out.push_str("    jnz .L_x64_fatal_have_msg\n");
        out.push_str("    mov %rbx, %rsi\n");
        out.push_str(".L_x64_fatal_have_msg:\n");
        out.push_str("    lea alya_fmt_runtime_err(%rip), %rdi\n");
        out.push_str("    xor %rax, %rax\n");
        out.push_str(&format!("    call {}printf\n", p));
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str(&format!("    call {}fflush\n", p));
        out.push_str("    mov $1, %rdi\n");
        out.push_str(&format!("    call {}exit\n\n", p));
    }

    // fn_rethrow: rethrows the in-flight value with its stored message.
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call alya_catch_block\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    call alya_catch_block\n");
    }
    out.push_str("    mov 3080(%rax), %rbx\n");
    out.push_str("    mov 3088(%rax), %r12\n");
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jnz .L_x64_rethrow_has_msg\n");
    out.push_str("    lea alya_str_unhandled_err(%rip), %rbx\n");
    out.push_str(".L_x64_rethrow_has_msg:\n");
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    mov %r12, %rdx\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    mov %r12, %rsi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    if is_win {
        out.push_str("    lea alya_str_div_zero(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_div_zero(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    if is_win {
        out.push_str("    lea alya_str_bounds(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_bounds(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    if is_win {
        out.push_str("    lea alya_str_null_unwrap(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_null_unwrap(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_null_field (struct field access on a null base,
    // alya-lang/alya#74): same catchable shape as the unwrap trap.
    out.push_str("alya_error_null_field:\n");
    if is_win {
        out.push_str("    lea alya_str_null_field(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_null_field(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_mixed_float (dynamic mixed int/float arithmetic through
    // untyped params, alya-lang/alya#39): the static checker rejects
    // provably-mixed reads; this catches what only tags can prove.
    out.push_str("alya_error_mixed_float:\n");
    if is_win {
        out.push_str("    lea alya_str_mixed_float(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_mixed_float(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // alya_error_nul_byte (chr(0), alya-lang/alya#129): Alya strings
    // are NUL-terminated, so code 0 has no string representation.
    // Loud by design: silently returning "" corrupted binary protocols.
    out.push_str("alya_error_nul_byte:\n");
    if is_win {
        out.push_str("    lea alya_str_nul_byte(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_nul_byte(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");

    // Trapped dynamic type errors (alya-lang/alya#154): calling a
    // non-function value, keys() on a non-container, and a small-int
    // reaching a string parameter all faulted. Same catchable shape
    // as the traps above; only provably-foreign values trap.
    out.push_str("alya_error_not_callable:\n");
    if is_win {
        out.push_str("    lea alya_str_not_callable(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_not_callable(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");
    out.push_str("alya_error_keys_type:\n");
    if is_win {
        out.push_str("    lea alya_str_keys_type(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_keys_type(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
    }
    out.push_str("    jmp fn_throw\n\n");
    out.push_str("alya_error_param_type:\n");
    if is_win {
        out.push_str("    lea alya_str_param_type(%rip), %rcx\n");
        out.push_str("    xor %edx, %edx\n");
    } else {
        out.push_str("    lea alya_str_param_type(%rip), %rdi\n");
        out.push_str("    xor %esi, %esi\n");
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
