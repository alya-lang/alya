use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // fn_alya_thread_proc - thread entry point
    out.push_str(".global fn_alya_thread_proc\n");
    out.push_str("fn_alya_thread_proc:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    push %rbx\n");
        out.push_str("    push %rsi\n");
        out.push_str("    push %rdi\n");
        out.push_str("    push %r12\n");
        out.push_str("    push %r13\n");
        out.push_str("    push %r14\n");
        out.push_str("    push %r15\n");
        out.push_str("    sub $40, %rsp\n");
        out.push_str("    mov %rcx, 32(%rsp)\n");  // save ctx
        out.push_str("    mov 32(%rcx), %rax\n");  // slot_id
        out.push_str("    mov %rax, %gs:0x28\n");  // store in Windows TEB ArbitraryUserPointer
        out.push_str("    mov 8(%rcx), %r11\n");   // func
        out.push_str("    mov 16(%rcx), %rcx\n");  // arg
        out.push_str("    call *%r11\n");
        out.push_str("    mov 32(%rsp), %r10\n");  // reload ctx
        out.push_str("    mov %rax, 24(%r10)\n");  // result
        out.push_str("    movq $0, %gs:0x28\n");   // clear ArbitraryUserPointer
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    add $40, %rsp\n");
        out.push_str("    pop %r15\n");
        out.push_str("    pop %r14\n");
        out.push_str("    pop %r13\n");
        out.push_str("    pop %r12\n");
        out.push_str("    pop %rdi\n");
        out.push_str("    pop %rsi\n");
        out.push_str("    pop %rbx\n");
    } else {
        out.push_str("    push %rbx\n");
        out.push_str("    push %r12\n");
        out.push_str("    push %r13\n");
        out.push_str("    push %r14\n");
        out.push_str("    push %r15\n");
        out.push_str("    sub $24, %rsp\n");
        out.push_str("    mov %rdi, 8(%rsp)\n");   // save ctx
        out.push_str("    mov 8(%rdi), %r11\n");   // func
        out.push_str("    mov 16(%rdi), %rdi\n");  // arg
        out.push_str("    call *%r11\n");
        out.push_str("    mov 8(%rsp), %r10\n");   // reload ctx
        out.push_str("    mov %rax, 24(%r10)\n");  // result
        out.push_str("    xor %rax, %rax\n");
        out.push_str("    add $24, %rsp\n");
        out.push_str("    pop %r15\n");
        out.push_str("    pop %r14\n");
        out.push_str("    pop %r13\n");
        out.push_str("    pop %r12\n");
        out.push_str("    pop %rbx\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_spawn
    out.push_str(".global fn___native_thread_spawn\n");
    out.push_str("fn___native_thread_spawn:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    if is_win {
        out.push_str("    sub $56, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");      // func
        out.push_str("    mov %rdx, %r13\n");      // arg
        out.push_str("    mov $40, %rcx\n");
        out.push_str("    call malloc\n");
        out.push_str("    mov %rax, %rbx\n");      // ctx
        out.push_str("    mov %r12, 8(%rbx)\n");
        out.push_str("    mov %r13, 16(%rbx)\n");
        out.push_str("    movq $0, 24(%rbx)\n");
        out.push_str("    mov $1, %rax\n");
        out.push_str("    lock xaddq %rax, alya_thread_slot_seq(%rip)\n");
        out.push_str("    inc %rax\n");
        out.push_str("    and $63, %rax\n");
        out.push_str("    mov %rax, 32(%rbx)\n");  // unique thread slot
        out.push_str("    xor %rcx, %rcx\n");      // lpThreadAttributes = NULL
        out.push_str("    xor %rdx, %rdx\n");      // dwStackSize = 0
        out.push_str("    lea fn_alya_thread_proc(%rip), %r8\n"); // lpStartAddress
        out.push_str("    mov %rbx, %r9\n");       // lpParameter
        out.push_str("    movq $0, 32(%rsp)\n");   // dwCreationFlags = 0
        out.push_str("    movq $0, 40(%rsp)\n");   // lpThreadId = NULL
        out.push_str("    call CreateThread\n");
        out.push_str("    mov %rax, 0(%rbx)\n");   // ctx->os_handle
        out.push_str("    mov %rbx, %rax\n");      // return ctx
        out.push_str("    add $56, %rsp\n");
    } else {
        out.push_str("    sub $24, %rsp\n");
        out.push_str("    mov %rdi, %r12\n");      // func
        out.push_str("    mov %rsi, %r13\n");      // arg
        out.push_str("    mov $32, %rdi\n");
        out.push_str(&format!("    call {}malloc\n", p));
        out.push_str("    mov %rax, %rbx\n");      // ctx
        out.push_str("    mov %r12, 8(%rbx)\n");
        out.push_str("    mov %r13, 16(%rbx)\n");
        out.push_str("    movq $0, 24(%rbx)\n");
        out.push_str("    mov %rbx, %rdi\n");      // thread*
        out.push_str("    xor %rsi, %rsi\n");      // attr = NULL
        out.push_str("    lea fn_alya_thread_proc(%rip), %rdx\n");
        out.push_str("    mov %rbx, %rcx\n");      // arg
        out.push_str(&format!("    call {}pthread_create\n", p));
        out.push_str("    mov %rbx, %rax\n");      // return ctx
        out.push_str("    add $24, %rsp\n");
    }
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_join
    out.push_str(".global fn___native_thread_join\n");
    out.push_str("fn___native_thread_join:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rcx, %rbx\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_join_null\n");
        out.push_str("    mov 0(%rbx), %rcx\n");   // os_handle
        out.push_str("    mov $0xFFFFFFFF, %edx\n"); // INFINITE
        out.push_str("    call WaitForSingleObject\n");
        out.push_str("    mov 0(%rbx), %rcx\n");
        out.push_str("    call CloseHandle\n");
        out.push_str("    mov 24(%rbx), %r12\n");  // result
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call free\n");
        out.push_str("    mov %r12, %rax\n");
        out.push_str("    jmp .L_x64_join_done\n");
    } else {
        out.push_str("    sub $16, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_join_null\n");
        out.push_str("    mov 0(%rbx), %rdi\n");   // pthread_t
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str(&format!("    call {}pthread_join\n", p));
        out.push_str("    mov 24(%rbx), %r12\n");  // result
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
        out.push_str("    mov %r12, %rax\n");
        out.push_str("    jmp .L_x64_join_done\n");
    }
    out.push_str(".L_x64_join_null:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_join_done:\n");
    if is_win {
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    add $16, %rsp\n");
    }
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_id
    out.push_str(".global fn___native_thread_id\n");
    out.push_str("fn___native_thread_id:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call GetCurrentThreadId\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str(&format!("    call {}pthread_self\n", p));
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_new
    out.push_str(".global fn___native_mutex_new\n");
    out.push_str("fn___native_mutex_new:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    xor %rcx, %rcx\n");      // lpMutexAttributes = NULL
        out.push_str("    xor %rdx, %rdx\n");      // bInitialOwner = FALSE
        out.push_str("    xor %r8, %r8\n");        // lpName = NULL
        out.push_str("    call CreateMutexA\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    push %rbx\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov $1, %rdi\n");
        out.push_str("    mov $64, %rsi\n");       // 64 bytes for pthread_mutex_t
        out.push_str(&format!("    call {}calloc\n", p));
        out.push_str("    mov %rax, %rbx\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str("    xor %rsi, %rsi\n");
        out.push_str(&format!("    call {}pthread_mutex_init\n", p));
        out.push_str("    mov %rbx, %rax\n");
        out.push_str("    add $8, %rsp\n");
        out.push_str("    pop %rbx\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_lock
    out.push_str(".global fn___native_mutex_lock\n");
    out.push_str("fn___native_mutex_lock:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov $0xFFFFFFFF, %edx\n"); // INFINITE
        out.push_str("    call WaitForSingleObject\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str(&format!("    call {}pthread_mutex_lock\n", p));
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_unlock
    out.push_str(".global fn___native_mutex_unlock\n");
    out.push_str("fn___native_mutex_unlock:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call ReleaseMutex\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str(&format!("    call {}pthread_mutex_unlock\n", p));
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_free
    out.push_str(".global fn___native_mutex_free\n");
    out.push_str("fn___native_mutex_free:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call CloseHandle\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    push %rbx\n");
        out.push_str("    sub $8, %rsp\n");
        out.push_str("    mov %rdi, %rbx\n");
        out.push_str("    test %rbx, %rbx\n");
        out.push_str("    jz .L_x64_mfree_done\n");
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}pthread_mutex_destroy\n", p));
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
        out.push_str(".L_x64_mfree_done:\n");
        out.push_str("    add $8, %rsp\n");
        out.push_str("    pop %rbx\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
