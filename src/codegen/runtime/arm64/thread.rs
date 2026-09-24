use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_alya_thread_proc - thread entry point
    out.push_str(".align 2\n");
    out.push_str(".global fn_alya_thread_proc\n");
    out.push_str("fn_alya_thread_proc:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");        // save ctx
    out.push_str("    ldr x8, [x0, #8]\n");         // func
    out.push_str("    ldr x0, [x0, #16]\n");        // arg
    out.push_str("    blr x8\n");                   // call func(arg)
    out.push_str("    ldr x1, [sp, #16]\n");        // reload ctx
    out.push_str("    str x0, [x1, #24]\n");        // ctx->result = x0
    out.push_str("    mov x0, #0\n");               // exit code 0
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_spawn
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_thread_spawn\n");
    out.push_str("fn___native_thread_spawn:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n");              // func
    out.push_str("    mov x20, x1\n");              // arg
    out.push_str("    sub sp, sp, #32\n");
    out.push_str("    mov x0, #32\n");              // 32 bytes
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    add sp, sp, #32\n");
    out.push_str("    mov x21, x0\n");              // ctx
    out.push_str("    str x19, [x21, #8]\n");       // ctx->func
    out.push_str("    str x20, [x21, #16]\n");      // ctx->arg
    out.push_str("    str xzr, [x21, #24]\n");      // ctx->result = 0
    if is_win {
        // Windows: CreateThread(NULL, 0, proc, ctx, 0, NULL).
        // Extra sub keeps the 32-byte home area below the call free.
        out.push_str("    sub sp, sp, #48\n");
        out.push_str("    mov x0, #0\n");
        out.push_str("    mov x1, #0\n");
        out.push_str("    adrp x2, fn_alya_thread_proc\n");
        out.push_str("    add x2, x2, :lo12:fn_alya_thread_proc\n");
        out.push_str("    mov x3, x21\n");
        out.push_str("    str xzr, [sp]\n");        // dwCreationFlags = 0
        out.push_str("    str xzr, [sp, #8]\n");    // lpThreadId = NULL
        out.push_str("    bl CreateThread\n");
        out.push_str("    add sp, sp, #48\n");
        out.push_str("    str x0, [x21]\n");        // ctx->os_handle
    } else {
    out.push_str("    mov x0, x21\n");              // thread*
    out.push_str("    mov x1, #0\n");               // attr = NULL
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    adrp x2, fn_alya_thread_proc@PAGE\n");
        out.push_str("    add x2, x2, fn_alya_thread_proc@PAGEOFF\n");
    } else {
        out.push_str("    adrp x2, fn_alya_thread_proc\n");
        out.push_str("    add x2, x2, :lo12:fn_alya_thread_proc\n");
    }
    out.push_str("    mov x3, x21\n");              // arg = ctx
    out.push_str(&format!("    bl {}pthread_create\n", p));
    }
    out.push_str("    mov x0, x21\n");              // return ctx handle
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_join
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_thread_join\n");
    out.push_str("fn___native_thread_join:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");              // ctx
    out.push_str("    cbz x19, .L_arm64_join_null\n");
    if is_win {
        // Windows: WaitForSingleObject(handle, INFINITE), CloseHandle.
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    ldr x0, [x19]\n");        // os_handle
        out.push_str("    movn w1, #0\n");          // INFINITE
        out.push_str("    bl WaitForSingleObject\n");
        out.push_str("    ldr x0, [x19]\n");
        out.push_str("    bl CloseHandle\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
    out.push_str("    ldr x0, [x19, #0]\n");        // pthread_t
    out.push_str("    mov x1, #0\n");               // retval = NULL
    out.push_str(&format!("    bl {}pthread_join\n", p));
    }
    out.push_str("    ldr x20, [x19, #24]\n");      // result
    out.push_str("    mov x0, x19\n");
    out.push_str("    sub sp, sp, #32\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str("    add sp, sp, #32\n");
    out.push_str("    mov x0, x20\n");              // return result
    out.push_str("    b .L_arm64_join_done\n");
    out.push_str(".L_arm64_join_null:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_join_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn___native_thread_id
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_thread_id\n");
    out.push_str("fn___native_thread_id:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    if is_win {
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    bl GetCurrentThreadId\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
        out.push_str(&format!("    bl {}pthread_self\n", p));
    }
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_new
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_mutex_new\n");
    out.push_str("fn___native_mutex_new:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x19, [sp, #16]\n");
    if is_win {
        // Windows: mutex IS the HANDLE (CreateMutexA), no extra storage.
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    mov x0, #0\n");           // lpMutexAttributes = NULL
        out.push_str("    mov x1, #0\n");           // bInitialOwner = FALSE
        out.push_str("    mov x2, #0\n");           // lpName = NULL
        out.push_str("    bl CreateMutexA\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
    out.push_str("    mov x0, #1\n");
    out.push_str("    mov x1, #64\n");              // 64 bytes for pthread_mutex_t
    out.push_str(&format!("    bl {}calloc\n", p));
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x0, x19\n");
    out.push_str("    mov x1, #0\n");
    out.push_str(&format!("    bl {}pthread_mutex_init\n", p));
    out.push_str("    mov x0, x19\n");              // return mutex ptr
    }
    out.push_str("    ldr x19, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_lock
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_mutex_lock\n");
    out.push_str("fn___native_mutex_lock:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    if is_win {
        // Windows: WaitForSingleObject(handle, INFINITE).
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    movn w1, #0\n");          // INFINITE
        out.push_str("    bl WaitForSingleObject\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
        out.push_str(&format!("    bl {}pthread_mutex_lock\n", p));
    }
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_unlock
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_mutex_unlock\n");
    out.push_str("fn___native_mutex_unlock:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    if is_win {
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    bl ReleaseMutex\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
        out.push_str(&format!("    bl {}pthread_mutex_unlock\n", p));
    }
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_mutex_free
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_mutex_free\n");
    out.push_str("fn___native_mutex_free:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x19, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    cbz x19, .L_arm64_mfree_done\n");
    if is_win {
        // Windows: the HANDLE itself is closed; nothing extra was allocated.
        out.push_str("    sub sp, sp, #32\n");
        out.push_str("    mov x0, x19\n");
        out.push_str("    bl CloseHandle\n");
        out.push_str("    add sp, sp, #32\n");
    } else {
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    bl {}pthread_mutex_destroy\n", p));
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    bl {}free\n", p));
    }
    out.push_str(".L_arm64_mfree_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldr x19, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");
}
