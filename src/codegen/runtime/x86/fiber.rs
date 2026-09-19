use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // fn_alya_fiber_init
    // =========================================================================
    out.push_str(".global fn_alya_fiber_init\n");
    out.push_str("fn_alya_fiber_init:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    cmpl $0, alya_fiber_current\n");
    out.push_str("    jne .L_x86_fib_init_done\n");
    out.push_str("    lea alya_fiber_main, %eax\n");
    out.push_str("    movl $0, 0(%eax)\n");          // id = 0
    out.push_str("    movl $1, 4(%eax)\n");          // state = 1 (RUNNING)
    out.push_str("    movl $0, 8(%eax)\n");          // stack_base = 0
    out.push_str("    movl $0, 12(%eax)\n");         // stack_size = 0
    out.push_str("    movl %esp, 16(%eax)\n");       // saved_sp
    out.push_str("    movl $0, 20(%eax)\n");         // fn_ptr = 0
    out.push_str("    movl $0, 24(%eax)\n");         // arg = 0
    out.push_str("    movl $0, 28(%eax)\n");         // result = 0
    out.push_str("    movl $0, 32(%eax)\n");         // parent = 0
    out.push_str("    movl $0, 36(%eax)\n");         // next = 0
    out.push_str("    movl %eax, alya_fiber_current\n");
    out.push_str("    movl $1, alya_fiber_active_count\n");
    out.push_str(".L_x86_fib_init_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_switch(from_fib, to_fib)
    // =========================================================================
    out.push_str(".global fn_alya_fiber_switch\n");
    out.push_str("fn_alya_fiber_switch:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %edi\n");
    out.push_str("    push %esi\n");
    out.push_str("    mov 20(%esp), %edx\n");        // from_fib
    out.push_str("    mov 24(%esp), %ecx\n");        // to_fib
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_fib_sw_no_save\n");
    out.push_str("    mov %esp, 16(%edx)\n");        // save current ESP
    out.push_str(".L_x86_fib_sw_no_save:\n");
    out.push_str("    mov 16(%ecx), %esp\n");        // restore target ESP
    out.push_str("    pop %esi\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_trampoline
    // =========================================================================
    out.push_str(".global fn_alya_fiber_trampoline\n");
    out.push_str("fn_alya_fiber_trampoline:\n");
    out.push_str("    mov alya_fiber_current, %ebx\n");
    out.push_str("    movl $1, 4(%ebx)\n");          // state = RUNNING (1)
    out.push_str("    push 24(%ebx)\n");             // push arg
    out.push_str("    call *20(%ebx)\n");            // call fn_ptr(arg)
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov alya_fiber_current, %ebx\n");
    out.push_str("    mov %eax, 28(%ebx)\n");        // result
    out.push_str("    movl $3, 4(%ebx)\n");          // state = COMPLETED (3)
    out.push_str("    call fn_alya_fiber_exit\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_exit
    // =========================================================================
    out.push_str(".global fn_alya_fiber_exit\n");
    out.push_str("fn_alya_fiber_exit:\n");
    out.push_str("    decl alya_fiber_active_count\n");
    out.push_str("    mov alya_fiber_runqueue_head, %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_fib_exit_to_main\n");

    // Pop head
    out.push_str("    mov 36(%edx), %eax\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_head\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_fib_exit_has_tail\n");
    out.push_str("    movl $0, alya_fiber_runqueue_tail\n");
    out.push_str(".L_x86_fib_exit_has_tail:\n");
    out.push_str("    movl $0, 36(%edx)\n");

    out.push_str("    mov alya_fiber_current, %eax\n");
    out.push_str("    mov %edx, alya_fiber_current\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_alya_fiber_switch\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    ret\n");

    out.push_str(".L_x86_fib_exit_to_main:\n");
    out.push_str("    mov alya_fiber_current, %eax\n");
    out.push_str("    lea alya_fiber_main, %edx\n");
    out.push_str("    cmp %eax, %edx\n");
    out.push_str("    je .L_x86_fib_exit_already_main\n");
    out.push_str("    mov %edx, alya_fiber_current\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_alya_fiber_switch\n");
    out.push_str("    add $8, %esp\n");
    out.push_str(".L_x86_fib_exit_already_main:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_spawn(func, arg)
    // =========================================================================
    out.push_str(".global fn___native_fiber_spawn\n");
    out.push_str("fn___native_fiber_spawn:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    mov 8(%ebp), %esi\n");         // func
    out.push_str("    mov 12(%ebp), %edi\n");        // arg
    out.push_str("    call fn_alya_fiber_init\n");

    // Stack check
    out.push_str("    mov alya_fiber_pool_head, %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_fib_alloc_stack\n");
    out.push_str("    mov (%eax), %edx\n");
    out.push_str("    mov %edx, alya_fiber_pool_head\n");
    out.push_str("    mov %eax, %ebx\n");            // stack_base
    out.push_str("    jmp .L_x86_fib_alloc_struct\n");

    out.push_str(".L_x86_fib_alloc_stack:\n");
    out.push_str("    push $16384\n");
    out.push_str(&format!("    call {}malloc\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %eax, %ebx\n");            // stack_base

    out.push_str(".L_x86_fib_alloc_struct:\n");
    out.push_str("    push $64\n");
    out.push_str(&format!("    call {}malloc\n", p));
    out.push_str("    add $4, %esp\n");              // %eax = fiber struct

    // Stack top setup: top = stack_base + 16384
    out.push_str("    lea 16384(%ebx), %edx\n");
    out.push_str("    and $-16, %edx\n");
    out.push_str("    lea fn_alya_fiber_trampoline, %ecx\n");
    out.push_str("    mov %ecx, -4(%edx)\n");        // return target
    out.push_str("    movl $0, -8(%edx)\n");         // ebp
    out.push_str("    movl $0, -12(%edx)\n");        // ebx
    out.push_str("    movl $0, -16(%edx)\n");        // edi
    out.push_str("    movl $0, -20(%edx)\n");        // esi
    out.push_str("    sub $20, %edx\n");             // saved_sp

    // Fields
    out.push_str("    incl alya_fiber_seq\n");
    out.push_str("    mov alya_fiber_seq, %ecx\n");
    out.push_str("    mov %ecx, 0(%eax)\n");         // id
    out.push_str("    movl $0, 4(%eax)\n");          // state = READY (0)
    out.push_str("    mov %ebx, 8(%eax)\n");         // stack_base
    out.push_str("    movl $16384, 12(%eax)\n");     // stack_size
    out.push_str("    mov %edx, 16(%eax)\n");        // saved_sp
    out.push_str("    mov %esi, 20(%eax)\n");        // fn_ptr
    out.push_str("    mov %edi, 24(%eax)\n");        // arg
    out.push_str("    movl $0, 28(%eax)\n");         // result
    out.push_str("    movl $0, 32(%eax)\n");         // parent
    out.push_str("    movl $0, 36(%eax)\n");         // next

    // Enqueue
    out.push_str("    mov alya_fiber_runqueue_tail, %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_fib_spawn_empty_q\n");
    out.push_str("    mov %eax, 36(%ecx)\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_tail\n");
    out.push_str("    jmp .L_x86_fib_spawn_enq_done\n");
    out.push_str(".L_x86_fib_spawn_empty_q:\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_head\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_tail\n");
    out.push_str(".L_x86_fib_spawn_enq_done:\n");
    out.push_str("    incl alya_fiber_active_count\n");
    out.push_str("    incl alya_fiber_count\n");

    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_yield
    // =========================================================================
    out.push_str(".global fn___native_fiber_yield\n");
    out.push_str("fn___native_fiber_yield:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov alya_fiber_runqueue_head, %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_fib_yd_zero\n");

    // Pop head
    out.push_str("    mov 36(%edx), %eax\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_head\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_fib_yd_has_tail\n");
    out.push_str("    movl $0, alya_fiber_runqueue_tail\n");
    out.push_str(".L_x86_fib_yd_has_tail:\n");
    out.push_str("    movl $0, 36(%edx)\n");

    // Re-enqueue current
    out.push_str("    mov alya_fiber_current, %eax\n");
    out.push_str("    cmpl $1, 4(%eax)\n");
    out.push_str("    jne .L_x86_fib_yd_no_reenq\n");
    out.push_str("    movl $0, 4(%eax)\n");          // state = READY
    out.push_str("    movl $0, 36(%eax)\n");
    out.push_str("    mov alya_fiber_runqueue_tail, %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jz .L_x86_fib_yd_empty_reenq\n");
    out.push_str("    mov %eax, 36(%ecx)\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_tail\n");
    out.push_str("    jmp .L_x86_fib_yd_no_reenq\n");
    out.push_str(".L_x86_fib_yd_empty_reenq:\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_head\n");
    out.push_str("    mov %eax, alya_fiber_runqueue_tail\n");

    out.push_str(".L_x86_fib_yd_no_reenq:\n");
    out.push_str("    mov %edx, alya_fiber_current\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call fn_alya_fiber_switch\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov alya_fiber_current, %eax\n");
    out.push_str("    movl $1, 4(%eax)\n");          // state = 1 (RUNNING)
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_fib_yd_done\n");
    out.push_str(".L_x86_fib_yd_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_fib_yd_done:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_join
    // =========================================================================
    out.push_str(".global fn___native_fiber_join\n");
    out.push_str("fn___native_fiber_join:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str("    test %ebx, %ebx\n");
    out.push_str("    jz .L_x86_fib_jn_null\n");
    out.push_str(".L_x86_fib_jn_loop:\n");
    out.push_str("    cmpl $3, 4(%ebx)\n");
    out.push_str("    je .L_x86_fib_jn_completed\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    jmp .L_x86_fib_jn_loop\n");
    out.push_str(".L_x86_fib_jn_completed:\n");
    out.push_str("    mov 28(%ebx), %eax\n");        // result
    // Recycle stack to pool
    out.push_str("    mov 8(%ebx), %edx\n");         // stack_base
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_fib_jn_free\n");
    out.push_str("    mov alya_fiber_pool_head, %ecx\n");
    out.push_str("    mov %ecx, (%edx)\n");
    out.push_str("    mov %edx, alya_fiber_pool_head\n");
    out.push_str(".L_x86_fib_jn_free:\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %ebx\n");
    out.push_str(&format!("    call {}free\n", p));
    out.push_str("    add $4, %esp\n");
    out.push_str("    pop %eax\n");
    out.push_str("    jmp .L_x86_fib_jn_done\n");
    out.push_str(".L_x86_fib_jn_null:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_fib_jn_done:\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_drain
    // =========================================================================
    out.push_str(".global fn___native_fiber_drain\n");
    out.push_str("fn___native_fiber_drain:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str(".L_x86_fib_drain_loop:\n");
    out.push_str("    cmpl $0, alya_fiber_runqueue_head\n");
    out.push_str("    je .L_x86_fib_drain_done\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    jmp .L_x86_fib_drain_loop\n");
    out.push_str(".L_x86_fib_drain_done:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_count, id, sleep
    // =========================================================================
    out.push_str(".global fn___native_fiber_count\n");
    out.push_str("fn___native_fiber_count:\n");
    out.push_str("    mov alya_fiber_active_count, %eax\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn___native_fiber_id\n");
    out.push_str("fn___native_fiber_id:\n");
    out.push_str("    mov alya_fiber_current, %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x86_fib_id_zero\n");
    out.push_str("    mov 0(%eax), %eax\n");
    out.push_str("    ret\n");
    out.push_str(".L_x86_fib_id_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn___native_fiber_sleep\n");
    out.push_str("fn___native_fiber_sleep:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%ebp), %ebx\n");
    out.push_str(".L_x86_fib_slp_loop:\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    cmpl $0, %ebx\n");
    out.push_str("    jle .L_x86_fib_slp_done\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    push $1\n");
        out.push_str("    call Sleep\n");
    } else {
        out.push_str("    push $1000\n");
        out.push_str(&format!("    call {}usleep\n", p));
        out.push_str("    add $4, %esp\n");
    }
    out.push_str("    decl %ebx\n");
    out.push_str("    jmp .L_x86_fib_slp_loop\n");
    out.push_str(".L_x86_fib_slp_done:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
