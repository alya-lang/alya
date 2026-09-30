use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_exit
    out.push_str("fn_exit:\n");
    out.push_str(&format!("    b {}exit\n\n", p));

    // Per-thread catch state (alya-lang/alya#65). Heap block layout
    // (zeroed via calloc): idx@0, handler[128]@8, sp[128]@1032,
    // bp[128]@2056, err_msg@3080, err_str@3088 (3096 bytes). The
    // key/FLS slot is created once via pthread_once / InitOnceExecuteOnce;
    // each thread's block is freed by the key destructor / FLS callback.
    out.push_str(".global alya_catch_init\n");
    out.push_str("alya_catch_init:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    if is_win {
        emit_adrp_add(out, "x0", "alya_catch_free", os);
        out.push_str("    bl FlsAlloc\n");
        emit_adrp_add(out, "x1", "alya_catch_fls", os);
        out.push_str("    str w0, [x1]\n");
        out.push_str("    mov w0, #1\n");
    } else {
        emit_adrp_add(out, "x0", "alya_catch_key", os);
        emit_adrp_add(out, "x1", "alya_catch_free", os);
        out.push_str(&format!("    bl {}pthread_key_create\n", p));
        out.push_str("    mov x0, #0\n");
    }
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // alya_catch_free: per-thread block destructor (never sees null).
    out.push_str(".global alya_catch_free\n");
    out.push_str("alya_catch_free:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    if is_win {
        out.push_str("    bl free\n");
    } else {
        out.push_str(&format!("    bl {}free\n", p));
    }
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // alya_catch_block: current thread's catch-state block (allocates
    // on first use). Exits the process on allocation failure.
    out.push_str(".global alya_catch_block\n");
    out.push_str("alya_catch_block:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    if is_win {
        emit_adrp_add(out, "x0", "alya_catch_once", os);
        emit_adrp_add(out, "x1", "alya_catch_init", os);
        out.push_str("    mov x2, #0\n");
        out.push_str("    mov x3, #0\n");
        out.push_str("    bl InitOnceExecuteOnce\n");
        emit_adrp_add(out, "x9", "alya_catch_fls", os);
        out.push_str("    ldr w0, [x9]\n");
        out.push_str("    bl FlsGetValue\n");
    } else {
        emit_adrp_add(out, "x0", "alya_catch_once", os);
        emit_adrp_add(out, "x1", "alya_catch_init", os);
        out.push_str(&format!("    bl {}pthread_once\n", p));
        emit_adrp_add(out, "x9", "alya_catch_key", os);
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    ldr x0, [x9]\n");
        } else {
            out.push_str("    ldr w0, [x9]\n");
        }
        out.push_str(&format!("    bl {}pthread_getspecific\n", p));
    }
    out.push_str("    cbnz x0, .L_arm_catch_block_ret\n");
    out.push_str("    mov x0, #1\n");
    out.push_str("    mov x1, #3096\n");
    if is_win {
        out.push_str("    bl calloc\n");
    } else {
        out.push_str(&format!("    bl {}calloc\n", p));
    }
    out.push_str("    cbz x0, .L_arm_catch_block_oom\n");
    out.push_str("    mov x19, x0\n");
    if is_win {
        emit_adrp_add(out, "x9", "alya_catch_fls", os);
        out.push_str("    ldr w0, [x9]\n");
        out.push_str("    mov x1, x19\n");
        out.push_str("    bl FlsSetValue\n");
    } else {
        emit_adrp_add(out, "x9", "alya_catch_key", os);
        if matches!(os, OperatingSystem::MacOS) {
            out.push_str("    ldr x0, [x9]\n");
        } else {
            out.push_str("    ldr w0, [x9]\n");
        }
        out.push_str("    mov x1, x19\n");
        out.push_str(&format!("    bl {}pthread_setspecific\n", p));
    }
    out.push_str("    mov x0, x19\n");
    out.push_str("    b .L_arm_catch_block_ret\n");
    out.push_str(".L_arm_catch_block_oom:\n");
    out.push_str("    mov w0, #1\n");
    if is_win {
        out.push_str("    bl exit\n");
    } else {
        out.push_str(&format!("    bl {}exit\n", p));
    }
    out.push_str(".L_arm_catch_block_ret:\n");
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // alya_try_begin(x0=handler, x1=sp, x2=bp): push one catch frame.
    out.push_str(".global alya_try_begin\n");
    out.push_str("alya_try_begin:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    out.push_str("    stp x21, x22, [sp, #-16]!\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    mov x20, x1\n");
    out.push_str("    mov x21, x2\n");
    out.push_str("    bl alya_catch_block\n");
    out.push_str("    mov x22, x0\n");
    out.push_str("    ldr x9, [x22]\n");
    out.push_str("    cmp x9, #128\n");
    out.push_str("    b.hs .L_arm_try_begin_oom\n");
    out.push_str("    add x10, x22, x9, lsl #3\n");
    out.push_str("    str x19, [x10, #8]\n");
    out.push_str("    str x20, [x10, #1032]\n");
    out.push_str("    str x21, [x10, #2056]\n");
    out.push_str("    add x9, x9, #1\n");
    out.push_str("    str x9, [x22]\n");
    out.push_str("    b .L_arm_try_begin_ret\n");
    out.push_str(".L_arm_try_begin_oom:\n");
    out.push_str("    mov w0, #1\n");
    if is_win {
        out.push_str("    bl exit\n");
    } else {
        out.push_str(&format!("    bl {}exit\n", p));
    }
    out.push_str(".L_arm_try_begin_ret:\n");
    out.push_str("    ldp x21, x22, [sp], #16\n");
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // alya_try_end: pop one catch frame.
    out.push_str(".global alya_try_end\n");
    out.push_str("alya_try_end:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    out.push_str("    bl alya_catch_block\n");
    out.push_str("    ldr x9, [x0]\n");
    out.push_str("    sub x9, x9, #1\n");
    out.push_str("    str x9, [x0]\n");
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // alya_catch_msg: in-flight error value for `catch` bindings.
    out.push_str(".global alya_catch_msg\n");
    out.push_str("alya_catch_msg:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    out.push_str("    bl alya_catch_block\n");
    out.push_str("    ldr x0, [x0, #3080]\n");
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_throw
    out.push_str(".global fn_throw\n");
    out.push_str("fn_throw:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    out.push_str("    mov x19, x0\n");
    // x1 carries the message text (or null) on entry; stash it in a
    // callee-saved reg across the block call. stp keeps sp aligned.
    out.push_str("    mov x21, x1\n");
    out.push_str("    stp x21, x22, [sp, #-16]!\n");
    out.push_str("    bl alya_catch_block\n");
    out.push_str("    ldp x21, x22, [sp], #16\n");
    out.push_str("    mov x20, x0\n");
    out.push_str("    str x19, [x20, #3080]\n");
    out.push_str("    str x21, [x20, #3088]\n");
    out.push_str("    ldr x10, [x20]\n");
    out.push_str("    cbz x10, .L_arm_fatal_throw\n");
    out.push_str("    sub x10, x10, #1\n");
    out.push_str("    str x10, [x20]\n");
    out.push_str("    add x11, x20, x10, lsl #3\n");
    out.push_str("    ldr x14, [x11, #8]\n");
    out.push_str("    ldr x13, [x11, #1032]\n");
    out.push_str("    ldr x29, [x11, #2056]\n");
    out.push_str("    mov sp, x13\n");
    out.push_str("    br x14\n");
    out.push_str(".L_arm_fatal_throw:\n");
    out.push_str("    mov x19, sp\n");
    out.push_str("    bic x19, x19, #15\n");
    out.push_str("    mov sp, x19\n");
    // Prefer the extracted struct-message text (block+3088) when throw
    // codegen set it; the raw value may be a struct pointer, not a string.
    out.push_str("    ldr x1, [x20, #3088]\n");
    out.push_str("    cbnz x1, .L_arm_fatal_have_msg\n");
    out.push_str("    ldr x1, [x20, #3080]\n");
    out.push_str(".L_arm_fatal_have_msg:\n");
    emit_adrp_add(out, "x0", "alya_fmt_runtime_err", os);
    if matches!(os, OperatingSystem::MacOS) {
        out.push_str("    sub sp, sp, #16\n");
        out.push_str("    str x1, [sp]\n");
        out.push_str(&format!("    bl {}printf\n", p));
        out.push_str("    add sp, sp, #16\n");
    } else {
        out.push_str(&format!("    bl {}printf\n", p));
    }
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    bl {}fflush\n", p));
    out.push_str("    mov w0, #1\n");
    out.push_str(&format!("    bl {}exit\n\n", p));

    // fn_rethrow: rethrows the in-flight value with its stored message.
    out.push_str(".global fn_rethrow\n");
    out.push_str("fn_rethrow:\n");
    out.push_str("    sub sp, sp, #16\n");
    out.push_str("    bl alya_catch_block\n");
    out.push_str("    add sp, sp, #16\n");
    out.push_str("    ldr x1, [x0, #3088]\n");
    out.push_str("    ldr x0, [x0, #3080]\n");
    out.push_str("    cbnz x0, .L_arm_rethrow_has_msg\n");
    emit_adrp_add(out, "x0", "alya_str_unhandled_err", os);
    out.push_str(".L_arm_rethrow_has_msg:\n");
    out.push_str("    b fn_throw\n\n");

    // alya_error_div_zero
    out.push_str("alya_error_div_zero:\n");
    emit_adrp_add(out, "x0", "alya_str_div_zero", os);
    out.push_str("    mov x1, #0\n");
    out.push_str("    b fn_throw\n\n");

    // alya_error_index_out_of_bounds
    out.push_str("alya_error_index_out_of_bounds:\n");
    emit_adrp_add(out, "x0", "alya_str_bounds", os);
    out.push_str("    mov x1, #0\n");
    out.push_str("    b fn_throw\n\n");

    // alya_error_null_unwrap (force unwrap of null, Chapter 19 §1.6)
    out.push_str("alya_error_null_unwrap:\n");
    emit_adrp_add(out, "x0", "alya_str_null_unwrap", os);
    out.push_str("    mov x1, #0\n");
    out.push_str("    b fn_throw\n\n");

    // alya_error_mixed_float (dynamic mixed int/float arithmetic,
    // alya-lang/alya#39; see the x64 note above).
    out.push_str("alya_error_mixed_float:\n");
    emit_adrp_add(out, "x0", "alya_str_mixed_float", os);
    out.push_str("    mov x1, #0\n");
    out.push_str("    b fn_throw\n\n");

    // fn_sleep
    out.push_str(".align 2\n");
    out.push_str(".global fn_sleep\n");
    out.push_str("fn_sleep:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x1, #1000\n");
    out.push_str("    mul x0, x0, x1\n");
    out.push_str(&format!("    bl {}usleep\n", p));
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_system_exec
    out.push_str(".align 2\n");
    out.push_str(".global fn_system_exec\n");
    out.push_str("fn_system_exec:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_sysexec_empty\n");
    out.push_str(&format!("    bl {}system\n", p));
    out.push_str("    b .L_arm64_sysexec_ret\n");
    out.push_str(".L_arm64_sysexec_empty:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_sysexec_ret:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_get_pid
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_get_pid\n");
    out.push_str("fn___native_get_pid:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(&format!("    bl {}getpid\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_get_cwd
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_get_cwd\n");
    out.push_str("fn___native_get_cwd:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    sub sp, sp, #1024\n");
    out.push_str("    mov x0, sp\n");
    out.push_str("    mov x1, #1024\n");
    out.push_str(&format!("    bl {}getcwd\n", p));
    out.push_str("    cbz x0, .L_arm64_gcwd_empty\n");
    out.push_str("    bl fn_str_clone\n");
    out.push_str("    b .L_arm64_gcwd_done\n");
    out.push_str(".L_arm64_gcwd_empty:\n");
    emit_adrp_add(out, "x0", "alya_str_empty", os);
    out.push_str(".L_arm64_gcwd_done:\n");
    out.push_str("    add sp, sp, #1024\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn___native_set_cwd
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_set_cwd\n");
    out.push_str("fn___native_set_cwd:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    cbz x0, .L_arm64_scwd_fail\n");
    out.push_str(&format!("    bl {}chdir\n", p));
    out.push_str("    cmp x0, #0\n");
    out.push_str("    cset x0, eq\n");
    out.push_str("    b .L_arm64_scwd_done\n");
    out.push_str(".L_arm64_scwd_fail:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_scwd_done:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");
}
