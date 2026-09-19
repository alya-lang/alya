use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // fn_alya_fiber_init
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_alya_fiber_init\n");
    out.push_str("fn_alya_fiber_init:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    cbnz x9, .L_arm64_fib_init_done\n");
    emit_adrp_add(out, "x9", "alya_fiber_main", os);
    out.push_str("    str xzr, [x9, #0]\n");         // id = 0
    out.push_str("    mov x10, #1\n");
    out.push_str("    str x10, [x9, #8]\n");         // state = 1 (RUNNING)
    out.push_str("    str xzr, [x9, #16]\n");        // stack_base = 0
    out.push_str("    str xzr, [x9, #24]\n");        // stack_size = 0
    out.push_str("    mov x11, sp\n");
    out.push_str("    str x11, [x9, #32]\n");        // saved_sp
    out.push_str("    str xzr, [x9, #40]\n");        // fn_ptr
    out.push_str("    str xzr, [x9, #48]\n");        // arg
    out.push_str("    str xzr, [x9, #56]\n");        // result
    out.push_str("    str xzr, [x9, #64]\n");        // parent
    out.push_str("    str xzr, [x9, #72]\n");        // next
    out.push_str("    str xzr, [x9, #80]\n");        // wait_data
    out.push_str("    str x9, [x8]\n");              // alya_fiber_current = &alya_fiber_main
    emit_adrp_add(out, "x8", "alya_fiber_active_count", os);
    out.push_str("    str x10, [x8]\n");             // active_count = 1
    out.push_str(".L_arm64_fib_init_done:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_switch(from_fib, to_fib)
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_alya_fiber_switch\n");
    out.push_str("fn_alya_fiber_switch:\n");
    out.push_str("    stp x19, x20, [sp, #-16]!\n");
    out.push_str("    stp x21, x22, [sp, #-16]!\n");
    out.push_str("    stp x23, x24, [sp, #-16]!\n");
    out.push_str("    stp x25, x26, [sp, #-16]!\n");
    out.push_str("    stp x27, x28, [sp, #-16]!\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x9, sp\n");
    out.push_str("    cbz x0, .L_arm64_fib_sw_no_save\n");
    out.push_str("    str x9, [x0, #32]\n");         // save current SP
    out.push_str(".L_arm64_fib_sw_no_save:\n");
    out.push_str("    ldr x9, [x1, #32]\n");         // load target SP
    out.push_str("    mov sp, x9\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ldp x27, x28, [sp], #16\n");
    out.push_str("    ldp x25, x26, [sp], #16\n");
    out.push_str("    ldp x23, x24, [sp], #16\n");
    out.push_str("    ldp x21, x22, [sp], #16\n");
    out.push_str("    ldp x19, x20, [sp], #16\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_trampoline
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_alya_fiber_trampoline\n");
    out.push_str("fn_alya_fiber_trampoline:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x19, [x8]\n");
    out.push_str("    mov x10, #1\n");
    out.push_str("    str x10, [x19, #8]\n");        // state = RUNNING (1)
    out.push_str("    ldr x8, [x19, #40]\n");        // fn_ptr
    out.push_str("    ldr x0, [x19, #48]\n");        // arg
    out.push_str("    blr x8\n");                    // call func(arg)
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x19, [x8]\n");
    out.push_str("    str x0, [x19, #56]\n");        // result
    out.push_str("    mov x10, #3\n");
    out.push_str("    str x10, [x19, #8]\n");        // state = COMPLETED (3)
    out.push_str("    bl fn_alya_fiber_exit\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_exit
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_alya_fiber_exit\n");
    out.push_str("fn_alya_fiber_exit:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    emit_adrp_add(out, "x8", "alya_fiber_active_count", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    sub x9, x9, #1\n");
    out.push_str("    str x9, [x8]\n");

    emit_adrp_add(out, "x8", "alya_fiber_runqueue_head", os);
    out.push_str("    ldr x1, [x8]\n");              // next_fib
    out.push_str("    cbz x1, .L_arm64_fib_exit_to_main\n");

    // Pop next fiber
    out.push_str("    ldr x9, [x1, #72]\n");
    out.push_str("    str x9, [x8]\n");
    out.push_str("    cbnz x9, .L_arm64_fib_exit_has_tail\n");
    emit_adrp_add(out, "x10", "alya_fiber_runqueue_tail", os);
    out.push_str("    str xzr, [x10]\n");
    out.push_str(".L_arm64_fib_exit_has_tail:\n");
    out.push_str("    str xzr, [x1, #72]\n");

    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x0, [x8]\n");
    out.push_str("    str x1, [x8]\n");
    out.push_str("    bl fn_alya_fiber_switch\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n");

    out.push_str(".L_arm64_fib_exit_to_main:\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x0, [x8]\n");
    emit_adrp_add(out, "x1", "alya_fiber_main", os);
    out.push_str("    cmp x0, x1\n");
    out.push_str("    b.eq .L_arm64_fib_exit_done\n");
    out.push_str("    str x1, [x8]\n");
    out.push_str("    bl fn_alya_fiber_switch\n");
    out.push_str(".L_arm64_fib_exit_done:\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_spawn(func, arg)
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_spawn\n");
    out.push_str("fn___native_fiber_spawn:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    stp x21, x22, [sp, #32]\n");
    out.push_str("    mov x19, x0\n");              // func
    out.push_str("    mov x20, x1\n");              // arg
    out.push_str("    bl fn_alya_fiber_init\n");

    // Stack allocation / pool check
    emit_adrp_add(out, "x8", "alya_fiber_pool_head", os);
    out.push_str("    ldr x21, [x8]\n");
    out.push_str("    cbz x21, .L_arm64_fib_alloc_new_stack\n");
    out.push_str("    ldr x9, [x21]\n");
    out.push_str("    str x9, [x8]\n");
    out.push_str("    b .L_arm64_fib_alloc_struct\n");

    out.push_str(".L_arm64_fib_alloc_new_stack:\n");
    out.push_str("    mov x0, #16384\n");
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    mov x21, x0\n");              // x21 = stack_base

    out.push_str(".L_arm64_fib_alloc_struct:\n");
    out.push_str("    mov x0, #128\n");
    out.push_str(&format!("    bl {}malloc\n", p));
    out.push_str("    mov x22, x0\n");              // x22 = fiber struct

    // Stack top setup: top = stack_base + 16384
    out.push_str("    add x9, x21, #16384\n");
    out.push_str("    and x9, x9, #-16\n");         // 16-byte align
    out.push_str("    stp xzr, xzr, [x9, #-16]!\n"); // top - 16: x19, x20
    out.push_str("    stp xzr, xzr, [x9, #-16]!\n"); // top - 32: x21, x22
    out.push_str("    stp xzr, xzr, [x9, #-16]!\n"); // top - 48: x23, x24
    out.push_str("    stp xzr, xzr, [x9, #-16]!\n"); // top - 64: x25, x26
    out.push_str("    stp xzr, xzr, [x9, #-16]!\n"); // top - 80: x27, x28
    emit_adrp_add(out, "x10", "fn_alya_fiber_trampoline", os);
    out.push_str("    stp xzr, x10, [x9, #-16]!\n"); // top - 96: x29=0, x30=trampoline

    // Struct field setup
    emit_adrp_add(out, "x8", "alya_fiber_seq", os);
    out.push_str("    ldr x10, [x8]\n");
    out.push_str("    add x10, x10, #1\n");
    out.push_str("    str x10, [x8]\n");
    out.push_str("    str x10, [x22, #0]\n");       // id
    out.push_str("    str xzr, [x22, #8]\n");       // state = READY (0)
    out.push_str("    str x21, [x22, #16]\n");      // stack_base
    out.push_str("    mov x10, #16384\n");
    out.push_str("    str x10, [x22, #24]\n");      // stack_size
    out.push_str("    str x9,  [x22, #32]\n");      // saved_sp
    out.push_str("    str x19, [x22, #40]\n");      // fn_ptr
    out.push_str("    str x20, [x22, #48]\n");      // arg
    out.push_str("    str xzr, [x22, #56]\n");      // result
    out.push_str("    str xzr, [x22, #64]\n");      // parent
    out.push_str("    str xzr, [x22, #72]\n");      // next
    out.push_str("    str xzr, [x22, #80]\n");      // wait_data

    // Enqueue
    emit_adrp_add(out, "x8", "alya_fiber_runqueue_tail", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    cbz x9, .L_arm64_fib_spawn_empty_q\n");
    out.push_str("    str x22, [x9, #72]\n");
    out.push_str("    str x22, [x8]\n");
    out.push_str("    b .L_arm64_fib_spawn_enq_done\n");
    out.push_str(".L_arm64_fib_spawn_empty_q:\n");
    emit_adrp_add(out, "x10", "alya_fiber_runqueue_head", os);
    out.push_str("    str x22, [x10]\n");
    out.push_str("    str x22, [x8]\n");
    out.push_str(".L_arm64_fib_spawn_enq_done:\n");
    emit_adrp_add(out, "x8", "alya_fiber_active_count", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    add x9, x9, #1\n");
    out.push_str("    str x9, [x8]\n");
    emit_adrp_add(out, "x8", "alya_fiber_count", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    add x9, x9, #1\n");
    out.push_str("    str x9, [x8]\n");

    out.push_str("    mov x0, x22\n");              // return fiber handle
    out.push_str("    ldp x21, x22, [sp, #32]\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_yield
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_yield\n");
    out.push_str("fn___native_fiber_yield:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    emit_adrp_add(out, "x8", "alya_fiber_runqueue_head", os);
    out.push_str("    ldr x1, [x8]\n");
    out.push_str("    cbz x1, .L_arm64_fib_yd_zero\n");

    // Pop head
    out.push_str("    ldr x9, [x1, #72]\n");
    out.push_str("    str x9, [x8]\n");
    out.push_str("    cbnz x9, .L_arm64_fib_yd_has_tail\n");
    emit_adrp_add(out, "x10", "alya_fiber_runqueue_tail", os);
    out.push_str("    str xzr, [x10]\n");
    out.push_str(".L_arm64_fib_yd_has_tail:\n");
    out.push_str("    str xzr, [x1, #72]\n");

    // Re-enqueue current
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x0, [x8]\n");
    out.push_str("    ldr x9, [x0, #8]\n");
    out.push_str("    cmp x9, #1\n");
    out.push_str("    b.ne .L_arm64_fib_yd_no_reenq\n");
    out.push_str("    str xzr, [x0, #8]\n");        // state = READY (0)
    emit_adrp_add(out, "x10", "alya_fiber_runqueue_tail", os);
    out.push_str("    ldr x11, [x10]\n");
    out.push_str("    cbz x11, .L_arm64_fib_yd_empty_reenq\n");
    out.push_str("    str x0, [x11, #72]\n");
    out.push_str("    str x0, [x10]\n");
    out.push_str("    b .L_arm64_fib_yd_no_reenq\n");
    out.push_str(".L_arm64_fib_yd_empty_reenq:\n");
    emit_adrp_add(out, "x12", "alya_fiber_runqueue_head", os);
    out.push_str("    str x0, [x12]\n");
    out.push_str("    str x0, [x10]\n");

    out.push_str(".L_arm64_fib_yd_no_reenq:\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    str x1, [x8]\n");              // current = next
    out.push_str("    bl fn_alya_fiber_switch\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x0, [x8]\n");
    out.push_str("    mov x9, #1\n");
    out.push_str("    str x9, [x0, #8]\n");          // state = 1 (RUNNING)
    out.push_str("    mov x0, #1\n");
    out.push_str("    b .L_arm64_fib_yd_done\n");
    out.push_str(".L_arm64_fib_yd_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_fib_yd_done:\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_join
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_join\n");
    out.push_str("fn___native_fiber_join:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x19, x20, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str("    cbz x19, .L_arm64_fib_jn_null\n");
    out.push_str(".L_arm64_fib_jn_loop:\n");
    out.push_str("    ldr x9, [x19, #8]\n");
    out.push_str("    cmp x9, #3\n");               // completed?
    out.push_str("    b.eq .L_arm64_fib_jn_completed\n");
    out.push_str("    bl fn___native_fiber_yield\n");
    out.push_str("    b .L_arm64_fib_jn_loop\n");
    out.push_str(".L_arm64_fib_jn_completed:\n");
    out.push_str("    ldr x20, [x19, #56]\n");      // result
    // Recycle stack to pool
    out.push_str("    ldr x9, [x19, #16]\n");       // stack_base
    out.push_str("    cbz x9, .L_arm64_fib_jn_free\n");
    emit_adrp_add(out, "x8", "alya_fiber_pool_head", os);
    out.push_str("    ldr x10, [x8]\n");
    out.push_str("    str x10, [x9]\n");
    out.push_str("    str x9, [x8]\n");
    out.push_str(".L_arm64_fib_jn_free:\n");
    out.push_str("    mov x0, x19\n");
    out.push_str(&format!("    bl {}free\n", p));
    out.push_str("    mov x0, x20\n");
    out.push_str("    b .L_arm64_fib_jn_done\n");
    out.push_str(".L_arm64_fib_jn_null:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_fib_jn_done:\n");
    out.push_str("    ldp x19, x20, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_drain
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_drain\n");
    out.push_str("fn___native_fiber_drain:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(".L_arm64_fib_drain_loop:\n");
    emit_adrp_add(out, "x8", "alya_fiber_runqueue_head", os);
    out.push_str("    ldr x9, [x8]\n");
    out.push_str("    cbz x9, .L_arm64_fib_drain_done\n");
    out.push_str("    bl fn___native_fiber_yield\n");
    out.push_str("    b .L_arm64_fib_drain_loop\n");
    out.push_str(".L_arm64_fib_drain_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_count, id, sleep
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_count\n");
    out.push_str("fn___native_fiber_count:\n");
    emit_adrp_add(out, "x8", "alya_fiber_active_count", os);
    out.push_str("    ldr x0, [x8]\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_id\n");
    out.push_str("fn___native_fiber_id:\n");
    emit_adrp_add(out, "x8", "alya_fiber_current", os);
    out.push_str("    ldr x8, [x8]\n");
    out.push_str("    cbz x8, .L_arm64_fib_id_zero\n");
    out.push_str("    ldr x0, [x8, #0]\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_fib_id_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn___native_fiber_sleep\n");
    out.push_str("fn___native_fiber_sleep:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x19, [sp, #16]\n");
    out.push_str("    mov x19, x0\n");
    out.push_str(".L_arm64_fib_slp_loop:\n");
    out.push_str("    bl fn___native_fiber_yield\n");
    out.push_str("    cmp x19, #0\n");
    out.push_str("    b.le .L_arm64_fib_slp_done\n");
    out.push_str("    mov x0, #1000\n");
    out.push_str(&format!("    bl {}usleep\n", p));
    out.push_str("    sub x19, x19, #1\n");
    out.push_str("    b .L_arm64_fib_slp_loop\n");
    out.push_str(".L_arm64_fib_slp_done:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ldr x19, [sp, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");
}
