use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);

    // fn_alya_thread_proc
    out.push_str(".global fn_alya_thread_proc\n");
    if is_win {
        out.push_str(".global _fn_alya_thread_proc@4\n");
        out.push_str("_fn_alya_thread_proc@4:\n");
    }
    out.push_str("fn_alya_thread_proc:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    mov 8(%ebp), %eax\n");     // %eax = ctx
    out.push_str("    mov %eax, -4(%ebp)\n");    // save ctx
    if is_win {
        out.push_str("    mov 16(%eax), %edx\n");   // slot_id
        out.push_str("    movl %edx, %fs:0x14\n");  // TEB ArbitraryUserPointer
    }
    out.push_str("    push 8(%eax)\n");          // push arg
    out.push_str("    call *4(%eax)\n");         // call func(arg)
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -4(%ebp), %edx\n");    // reload ctx
    out.push_str("    mov %eax, 12(%edx)\n");    // ctx->result = return value
    if is_win {
        out.push_str("    movl $0, %fs:0x14\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    if is_win {
        out.push_str("    ret $4\n\n");          // stdcall ThreadProc cleans up 4 bytes
    } else {
        out.push_str("    ret\n\n");
    }

    // fn___native_thread_spawn
    out.push_str(".global fn___native_thread_spawn\n");
    out.push_str("fn___native_thread_spawn:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");      // func
    out.push_str("    mov 12(%ebp), %edi\n");     // arg
    out.push_str("    push $20\n");               // 20 bytes for ThreadContext (includes slot_id)
    out.push_str("    call malloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %ebx\n");         // ctx
    out.push_str("    mov %esi, 4(%ebx)\n");      // ctx->func
    out.push_str("    mov %edi, 8(%ebx)\n");      // ctx->arg
    out.push_str("    movl $0, 12(%ebx)\n");      // ctx->result
    if is_win {
        out.push_str("    movl $1, %eax\n");
        out.push_str("    lock xaddl %eax, alya_thread_slot_seq\n");
        out.push_str("    incl %eax\n");
        out.push_str("    andl $63, %eax\n");
        out.push_str("    movl %eax, 16(%ebx)\n");  // ctx->slot_id
        out.push_str("    push $0\n");            // lpThreadId
        out.push_str("    push $0\n");            // dwCreationFlags
        out.push_str("    push %ebx\n");          // lpParameter
        out.push_str("    push $fn_alya_thread_proc\n"); // lpStartAddress
        out.push_str("    push $0\n");            // dwStackSize
        out.push_str("    push $0\n");            // lpThreadAttributes
        out.push_str("    call CreateThread\n");
        out.push_str("    mov %eax, 0(%ebx)\n");  // ctx->os_handle
    } else {
        out.push_str("    push %ebx\n");          // arg = ctx
        out.push_str("    push $fn_alya_thread_proc\n");
        out.push_str("    push $0\n");            // attr = NULL
        out.push_str("    push %ebx\n");          // thread*
        out.push_str("    call pthread_create\n");
        out.push_str("    add $16, %esp\n");
    }
    out.push_str("    mov %ebx, %eax\n");         // return ctx
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_join
    out.push_str(".global fn___native_thread_join\n");
    out.push_str("fn___native_thread_join:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_join_null\n");
    if is_win {
        out.push_str("    push $0xFFFFFFFF\n");   // INFINITE
        out.push_str("    push 0(%ebx)\n");       // os_handle
        out.push_str("    call WaitForSingleObject\n");
        out.push_str("    push 0(%ebx)\n");
        out.push_str("    call CloseHandle\n");
    } else {
        out.push_str("    push $0\n");            // retval = NULL
        out.push_str("    push 0(%ebx)\n");       // pthread_t
        out.push_str("    call pthread_join\n");
        out.push_str("    add $8, %esp\n");
    }
    out.push_str("    mov 12(%ebx), %esi\n");     // result
    out.push_str("    push %ebx\n");
    out.push_str("    call free\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %esi, %eax\n");
    out.push_str("    jmp .L_x86_join_done\n");
    out.push_str(".L_x86_join_null:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_join_done:\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_id: Linux x86 uses the gettid syscall (224)
    // instead of pthread_self: a pthread_t pointer is usually negative
    // as i32 (high addresses), while kernel TIDs are small positives
    // (the `tid > 0` contract). Raw int $0x80 needs no libc symbol.
    out.push_str(".global fn___native_thread_id\n");
    out.push_str("fn___native_thread_id:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    call GetCurrentThreadId\n");
    } else {
        out.push_str("    mov $224, %eax\n");
        out.push_str("    int $0x80\n");
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_new
    out.push_str(".global fn___native_mutex_new\n");
    out.push_str("fn___native_mutex_new:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    push $0\n");
        out.push_str("    push $0\n");
        out.push_str("    push $0\n");
        out.push_str("    call CreateMutexA\n");
    } else {
        out.push_str("    push $40\n");           // sizeof(pthread_mutex_t) on 32-bit x86 is 24..40 bytes
        out.push_str("    push $1\n");
        out.push_str("    call calloc\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    push %eax\n");          // save ptr
        out.push_str("    push $0\n");
        out.push_str("    push %eax\n");
        out.push_str("    call pthread_mutex_init\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    pop %eax\n");           // restore ptr
    }
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_lock
    out.push_str(".global fn___native_mutex_lock\n");
    out.push_str("fn___native_mutex_lock:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    push $0xFFFFFFFF\n");
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call WaitForSingleObject\n");
    } else {
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call pthread_mutex_lock\n");
        out.push_str("    add $4, %esp\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_unlock
    out.push_str(".global fn___native_mutex_unlock\n");
    out.push_str("fn___native_mutex_unlock:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call ReleaseMutex\n");
    } else {
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call pthread_mutex_unlock\n");
        out.push_str("    add $4, %esp\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_free
    out.push_str(".global fn___native_mutex_free\n");
    out.push_str("fn___native_mutex_free:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    if is_win {
        out.push_str("    push 8(%ebp)\n");
        out.push_str("    call CloseHandle\n");
    } else {
        out.push_str("    mov 8(%ebp), %eax\n");
        out.push_str("    test %eax, %eax\n");
        out.push_str("    jz .L_x86_mfree_done\n");
        out.push_str("    push %eax\n");
        out.push_str("    call pthread_mutex_destroy\n");
        out.push_str("    call free\n");
        out.push_str("    add $4, %esp\n");
        out.push_str(".L_x86_mfree_done:\n");
    }
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
