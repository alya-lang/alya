use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };

    // =========================================================================
    // fn_alya_fiber_init
    // Initializes the main fiber context and scheduler if not yet initialized.
    // =========================================================================
    out.push_str(".global fn_alya_fiber_init\n");
    out.push_str("fn_alya_fiber_init:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    cmpq $0, alya_fiber_current(%rip)\n");
    out.push_str("    jne .L_x64_fib_init_done\n");
    out.push_str("    lea alya_fiber_main(%rip), %rax\n");
    out.push_str("    movq $0, 0(%rax)\n");          // id = 0
    out.push_str("    movq $1, 8(%rax)\n");          // state = 1 (RUNNING)
    if is_win {
        out.push_str("    mov %gs:0x10, %rdx\n");
        out.push_str("    mov %rdx, 16(%rax)\n");     // stack_base = TEB StackLimit
        out.push_str("    mov %gs:0x08, %rdx\n");
        out.push_str("    mov %rdx, 24(%rax)\n");     // stack_size = TEB StackBase
        out.push_str("    mov %gs:0x1478, %rdx\n");
        out.push_str("    mov %rdx, 80(%rax)\n");     // wait_data = DeallocationStack
    } else {
        out.push_str("    movq $0, 16(%rax)\n");      // stack_base = 0
        out.push_str("    movq $0, 24(%rax)\n");      // stack_size = 0
    }
    out.push_str("    movq %rsp, 32(%rax)\n");       // saved_sp
    out.push_str("    movq $0, 40(%rax)\n");         // fn_ptr = 0
    out.push_str("    movq $0, 48(%rax)\n");         // arg = 0
    out.push_str("    movq $0, 56(%rax)\n");         // result = 0
    out.push_str("    movq $0, 64(%rax)\n");         // parent = 0
    out.push_str("    movq $0, 72(%rax)\n");         // next = 0
    out.push_str("    movq %rax, alya_fiber_current(%rip)\n");
    out.push_str("    movq $1, alya_fiber_active_count(%rip)\n");
    out.push_str(".L_x64_fib_init_done:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_switch(from_fib, to_fib)
    // Low-level callee-saved register context switch between two fibers.
    // =========================================================================
    out.push_str(".global fn_alya_fiber_switch\n");
    out.push_str("fn_alya_fiber_switch:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    out.push_str("    push %r15\n");
    out.push_str("    push %rdi\n");
    out.push_str("    push %rsi\n");
    out.push_str("    sub $8, %rsp\n");              // 16-byte stack alignment
    if is_win {
        out.push_str("    mov %rcx, %r10\n");        // from_fib
        out.push_str("    mov %rdx, %r11\n");        // to_fib
    } else {
        out.push_str("    mov %rdi, %r10\n");        // from_fib
        out.push_str("    mov %rsi, %r11\n");        // to_fib
    }
    out.push_str("    test %r10, %r10\n");
    out.push_str("    jz .L_x64_fib_sw_no_save\n");
    out.push_str("    mov %rsp, 32(%r10)\n");        // save current RSP
    out.push_str(".L_x64_fib_sw_no_save:\n");
    out.push_str("    mov 32(%r11), %rsp\n");        // restore target RSP
    if is_win {
        // Update TEB StackLimit & StackBase for target fiber
        out.push_str("    mov 16(%r11), %rax\n");    // to_fib->stack_base
        out.push_str("    test %rax, %rax\n");
        out.push_str("    jz .L_x64_fib_sw_no_teb\n");
        out.push_str("    mov %rax, %gs:0x10\n");    // TEB StackLimit
        out.push_str("    mov 24(%r11), %rdx\n");    // stack_size (or StackBase for main)
        out.push_str("    cmpq $0, 0(%r11)\n");      // main fiber id == 0?
        out.push_str("    je .L_x64_fib_sw_teb_main\n");
        out.push_str("    add %rax, %rdx\n");        // top = stack_base + stack_size
        out.push_str("    mov %rdx, %gs:0x08\n");    // TEB StackBase
        out.push_str("    mov 80(%r11), %rax\n");
        out.push_str("    mov %rax, %gs:0x1478\n");  // TEB DeallocationStack
        out.push_str("    jmp .L_x64_fib_sw_no_teb\n");
        out.push_str(".L_x64_fib_sw_teb_main:\n");
        out.push_str("    mov %rdx, %gs:0x08\n");    // TEB StackBase for main
        out.push_str("    mov 80(%r11), %rax\n");
        out.push_str("    mov %rax, %gs:0x1478\n");  // TEB DeallocationStack for main
        out.push_str(".L_x64_fib_sw_no_teb:\n");
    }
    out.push_str("    add $8, %rsp\n");
    out.push_str("    pop %rsi\n");
    out.push_str("    pop %rdi\n");
    out.push_str("    pop %r15\n");
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_alya_fiber_trampoline
    // First entry trampoline executed when a new fiber is scheduled.
    // =========================================================================
    out.push_str(".global fn_alya_fiber_trampoline\n");
    out.push_str("fn_alya_fiber_trampoline:\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");         // 32-byte shadow space, keeps RSP 16-byte aligned
        out.push_str("    mov alya_fiber_current(%rip), %rbx\n");
        out.push_str("    movq $1, 8(%rbx)\n");      // state = 1 (RUNNING)
        out.push_str("    mov 40(%rbx), %r11\n");    // fn_ptr
        out.push_str("    mov 48(%rbx), %rcx\n");    // arg
        out.push_str("    call *%r11\n");
        out.push_str("    mov alya_fiber_current(%rip), %rbx\n");
        out.push_str("    mov %rax, 56(%rbx)\n");    // result
        out.push_str("    movq $3, 8(%rbx)\n");      // state = 3 (COMPLETED)
        out.push_str("    add $32, %rsp\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call fn_alya_fiber_exit\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    ret\n\n");
    } else {
        out.push_str("    sub $16, %rsp\n");         // keeps RSP 16-byte aligned
        out.push_str("    mov alya_fiber_current(%rip), %rbx\n");
        out.push_str("    movq $1, 8(%rbx)\n");      // state = 1 (RUNNING)
        out.push_str("    mov 40(%rbx), %r11\n");    // fn_ptr
        out.push_str("    mov 48(%rbx), %rdi\n");    // arg
        out.push_str("    call *%r11\n");
        out.push_str("    mov alya_fiber_current(%rip), %rbx\n");
        out.push_str("    mov %rax, 56(%rbx)\n");    // result
        out.push_str("    movq $3, 8(%rbx)\n");      // state = 3 (COMPLETED)
        out.push_str("    add $16, %rsp\n");
        out.push_str("    call fn_alya_fiber_exit\n");
        out.push_str("    ret\n\n");
    }

    // =========================================================================
    // fn_alya_fiber_exit
    // Called when a fiber completes execution.
    // Switches to the next available fiber or the main scheduler.
    // =========================================================================
    out.push_str(".global fn_alya_fiber_exit\n");
    out.push_str("fn_alya_fiber_exit:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    sub $32, %rsp\n");
    }
    out.push_str("    decq alya_fiber_active_count(%rip)\n");

    // Acquire lock to pop next fiber
    out.push_str(".L_x64_fib_exit_lock:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    xchg %eax, alya_fiber_lock(%rip)\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x64_fib_exit_have_lock\n");
    out.push_str("    pause\n");
    out.push_str("    jmp .L_x64_fib_exit_lock\n");

    out.push_str(".L_x64_fib_exit_have_lock:\n");
    out.push_str("    mov alya_fiber_runqueue_head(%rip), %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jz .L_x64_fib_exit_to_main\n");

    // Pop head
    out.push_str("    mov 72(%rdx), %rax\n");
    out.push_str("    mov %rax, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_fib_exit_has_tail\n");
    out.push_str("    movq $0, alya_fiber_runqueue_tail(%rip)\n");
    out.push_str(".L_x64_fib_exit_has_tail:\n");
    out.push_str("    movq $0, 72(%rdx)\n");
    out.push_str("    movl $0, alya_fiber_lock(%rip)\n");

    // Switch from completed fiber to next ready fiber (%rdx)
    out.push_str("    mov alya_fiber_current(%rip), %rcx\n");
    out.push_str("    mov %rdx, alya_fiber_current(%rip)\n");
    if is_win {
        out.push_str("    call fn_alya_fiber_switch\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
        out.push_str("    call fn_alya_fiber_switch\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");

    out.push_str(".L_x64_fib_exit_to_main:\n");
    out.push_str("    movl $0, alya_fiber_lock(%rip)\n");
    out.push_str("    mov alya_fiber_current(%rip), %rcx\n");
    out.push_str("    lea alya_fiber_main(%rip), %rdx\n");
    out.push_str("    cmp %rcx, %rdx\n");
    out.push_str("    je .L_x64_fib_exit_already_main\n");
    out.push_str("    mov %rdx, alya_fiber_current(%rip)\n");
    if is_win {
        out.push_str("    call fn_alya_fiber_switch\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
        out.push_str("    call fn_alya_fiber_switch\n");
    }
    out.push_str(".L_x64_fib_exit_already_main:\n");
    if is_win {
        out.push_str("    add $32, %rsp\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_spawn(func, arg)
    // Allocates and enqueues a new lightweight green fiber.
    // =========================================================================
    out.push_str(".global fn___native_fiber_spawn\n");
    out.push_str("fn___native_fiber_spawn:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    push %r13\n");
    out.push_str("    push %r14\n");
    if is_win {
        out.push_str("    sub $48, %rsp\n");
        out.push_str("    mov %rcx, %r12\n");        // func
        out.push_str("    mov %rdx, %r13\n");        // arg
    } else {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    mov %rdi, %r12\n");        // func
        out.push_str("    mov %rsi, %r13\n");        // arg
    }

    out.push_str("    call fn_alya_fiber_init\n");

    // Check stack pool
    out.push_str("    mov alya_fiber_pool_head(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fib_alloc_new_stack\n");
    // Reuse stack from pool
    out.push_str("    mov (%rax), %rdx\n");
    out.push_str("    mov %rdx, alya_fiber_pool_head(%rip)\n");
    out.push_str("    mov %rax, %r14\n");            // r14 = stack_base
    out.push_str("    jmp .L_x64_fib_alloc_struct\n");

    out.push_str(".L_x64_fib_alloc_new_stack:\n");
    if is_win {
        out.push_str("    mov $16384, %rcx\n");
        out.push_str("    call malloc\n");
    } else {
        out.push_str("    mov $16384, %rdi\n");
        out.push_str(&format!("    call {}malloc\n", p));
    }
    out.push_str("    mov %rax, %r14\n");            // r14 = stack_base

    out.push_str(".L_x64_fib_alloc_struct:\n");
    if is_win {
        out.push_str("    mov $128, %rcx\n");
        out.push_str("    call malloc\n");
    } else {
        out.push_str("    mov $128, %rdi\n");
        out.push_str(&format!("    call {}malloc\n", p));
    }
    out.push_str("    mov %rax, %rbx\n");            // rbx = fiber struct

    // Setup fiber stack frame
    out.push_str("    lea 16384(%r14), %rax\n");     // top of stack
    out.push_str("    and $-16, %rax\n");            // 16-byte align top
    out.push_str("    lea fn_alya_fiber_trampoline(%rip), %rdx\n");
    out.push_str("    mov %rdx, -8(%rax)\n");        // return address -> trampoline
    out.push_str("    movq $0, -16(%rax)\n");        // rbp
    out.push_str("    movq $0, -24(%rax)\n");        // rbx
    out.push_str("    movq $0, -32(%rax)\n");        // r12
    out.push_str("    movq $0, -40(%rax)\n");        // r13
    out.push_str("    movq $0, -48(%rax)\n");        // r14
    out.push_str("    movq $0, -56(%rax)\n");        // r15
    out.push_str("    movq $0, -64(%rax)\n");        // rdi
    out.push_str("    movq $0, -72(%rax)\n");        // rsi
    out.push_str("    movq $0, -80(%rax)\n");        // alignment pad
    out.push_str("    sub $80, %rax\n");             // saved_sp

    // Setup fiber struct fields
    out.push_str("    mov $1, %rdx\n");
    out.push_str("    lock xaddq %rdx, alya_fiber_seq(%rip)\n");
    out.push_str("    inc %rdx\n");
    out.push_str("    mov %rdx, 0(%rbx)\n");         // id
    out.push_str("    movq $0, 8(%rbx)\n");          // state = READY (0)
    out.push_str("    mov %r14, 16(%rbx)\n");        // stack_base
    out.push_str("    movq $16384, 24(%rbx)\n");     // stack_size
    out.push_str("    mov %rax, 32(%rbx)\n");        // saved_sp
    out.push_str("    mov %r12, 40(%rbx)\n");        // fn_ptr
    out.push_str("    mov %r13, 48(%rbx)\n");        // arg
    out.push_str("    movq $0, 56(%rbx)\n");         // result
    out.push_str("    movq $0, 64(%rbx)\n");         // parent
    out.push_str("    movq $0, 72(%rbx)\n");         // next
    out.push_str("    movq $0, 80(%rbx)\n");         // wait_data

    // Enqueue to runqueue tail
    out.push_str(".L_x64_fib_spawn_lock:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    xchg %eax, alya_fiber_lock(%rip)\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x64_fib_spawn_enq\n");
    out.push_str("    pause\n");
    out.push_str("    jmp .L_x64_fib_spawn_lock\n");

    out.push_str(".L_x64_fib_spawn_enq:\n");
    out.push_str("    mov alya_fiber_runqueue_tail(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fib_spawn_empty_q\n");
    out.push_str("    mov %rbx, 72(%rax)\n");
    out.push_str("    mov %rbx, alya_fiber_runqueue_tail(%rip)\n");
    out.push_str("    jmp .L_x64_fib_spawn_enq_done\n");
    out.push_str(".L_x64_fib_spawn_empty_q:\n");
    out.push_str("    mov %rbx, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    mov %rbx, alya_fiber_runqueue_tail(%rip)\n");
    out.push_str(".L_x64_fib_spawn_enq_done:\n");
    out.push_str("    incq alya_fiber_active_count(%rip)\n");
    out.push_str("    incq alya_fiber_count(%rip)\n");
    out.push_str("    movl $0, alya_fiber_lock(%rip)\n");

    out.push_str("    mov %rbx, %rax\n");            // return fiber handle
    if is_win {
        out.push_str("    add $48, %rsp\n");
    } else {
        out.push_str("    add $32, %rsp\n");
    }
    out.push_str("    pop %r14\n");
    out.push_str("    pop %r13\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_yield
    // Cooperatively yields CPU to the next ready fiber in the runqueue.
    // Returns 1 if switched, 0 if runqueue was empty.
    // =========================================================================
    out.push_str(".global fn___native_fiber_yield\n");
    out.push_str("fn___native_fiber_yield:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");

    out.push_str("    cmpq $0, alya_fiber_current(%rip)\n");
    out.push_str("    jne .L_x64_fib_yd_check_q\n");
    out.push_str("    call fn_alya_fiber_init\n");

    out.push_str(".L_x64_fib_yd_check_q:\n");
    out.push_str("    cmpq $0, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    je .L_x64_fib_yd_ret_zero\n");

    // Acquire spinlock
    out.push_str(".L_x64_fib_yd_lock:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    xchg %eax, alya_fiber_lock(%rip)\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz .L_x64_fib_yd_have_lock\n");
    out.push_str("    pause\n");
    out.push_str("    jmp .L_x64_fib_yd_lock\n");

    out.push_str(".L_x64_fib_yd_have_lock:\n");
    out.push_str("    mov alya_fiber_runqueue_head(%rip), %rdx\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str("    jnz .L_x64_fib_yd_pop\n");
    out.push_str("    movl $0, alya_fiber_lock(%rip)\n");
    out.push_str("    jmp .L_x64_fib_yd_ret_zero\n");

    out.push_str(".L_x64_fib_yd_pop:\n");
    out.push_str("    mov 72(%rdx), %rax\n");
    out.push_str("    mov %rax, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_fib_yd_has_tail\n");
    out.push_str("    movq $0, alya_fiber_runqueue_tail(%rip)\n");
    out.push_str(".L_x64_fib_yd_has_tail:\n");
    out.push_str("    movq $0, 72(%rdx)\n");

    // Re-enqueue current fiber if state == 1 (RUNNING)
    out.push_str("    mov alya_fiber_current(%rip), %rcx\n");
    out.push_str("    cmpq $1, 8(%rcx)\n");
    out.push_str("    jne .L_x64_fib_yd_no_reenq\n");
    out.push_str("    movq $0, 8(%rcx)\n");         // state = 0 (READY)
    out.push_str("    movq $0, 72(%rcx)\n");        // next = 0
    out.push_str("    mov alya_fiber_runqueue_tail(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fib_yd_empty_reenq\n");
    out.push_str("    mov %rcx, 72(%rax)\n");
    out.push_str("    mov %rcx, alya_fiber_runqueue_tail(%rip)\n");
    out.push_str("    jmp .L_x64_fib_yd_no_reenq\n");
    out.push_str(".L_x64_fib_yd_empty_reenq:\n");
    out.push_str("    mov %rcx, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    mov %rcx, alya_fiber_runqueue_tail(%rip)\n");

    out.push_str(".L_x64_fib_yd_no_reenq:\n");
    out.push_str("    movl $0, alya_fiber_lock(%rip)\n");

    // Switch: rcx = current, rdx = next
    out.push_str("    mov %rdx, alya_fiber_current(%rip)\n");
    if is_win {
        out.push_str("    call fn_alya_fiber_switch\n");
    } else {
        out.push_str("    mov %rcx, %rdi\n");
        out.push_str("    mov %rdx, %rsi\n");
        out.push_str("    call fn_alya_fiber_switch\n");
    }
    out.push_str("    mov alya_fiber_current(%rip), %rax\n");
    out.push_str("    movq $1, 8(%rax)\n");          // state = 1 (RUNNING)
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_fib_yd_done\n");

    out.push_str(".L_x64_fib_yd_ret_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fib_yd_done:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_join(fiber)
    // Cooperatively waits for the target fiber to complete, frees its metadata,
    // and returns the fiber function's return value.
    // =========================================================================
    out.push_str(".global fn___native_fiber_join\n");
    out.push_str("fn___native_fiber_join:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    push %r12\n");
    out.push_str("    sub $32, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str("    test %rbx, %rbx\n");
    out.push_str("    jz .L_x64_fib_jn_null\n");

    out.push_str(".L_x64_fib_jn_loop:\n");
    out.push_str("    cmpq $3, 8(%rbx)\n");          // completed?
    out.push_str("    je .L_x64_fib_jn_completed\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    jmp .L_x64_fib_jn_loop\n");

    out.push_str(".L_x64_fib_jn_completed:\n");
    out.push_str("    mov 56(%rbx), %r12\n");        // result

    // Recycle stack to pool
    out.push_str("    mov 16(%rbx), %r9\n");         // stack_base
    out.push_str("    test %r9, %r9\n");
    out.push_str("    jz .L_x64_fib_jn_free_struct\n");
    out.push_str("    mov alya_fiber_pool_head(%rip), %rax\n");
    out.push_str("    mov %rax, (%r9)\n");
    out.push_str("    mov %r9, alya_fiber_pool_head(%rip)\n");

    out.push_str(".L_x64_fib_jn_free_struct:\n");
    // Free the fiber struct
    if is_win {
        out.push_str("    mov %rbx, %rcx\n");
        out.push_str("    call free\n");
    } else {
        out.push_str("    mov %rbx, %rdi\n");
        out.push_str(&format!("    call {}free\n", p));
    }
    out.push_str("    mov %r12, %rax\n");
    out.push_str("    jmp .L_x64_fib_jn_done\n");

    out.push_str(".L_x64_fib_jn_null:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_fib_jn_done:\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    pop %r12\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_drain
    // Drains and executes all pending ready fibers until runqueue is empty.
    // =========================================================================
    out.push_str(".global fn___native_fiber_drain\n");
    out.push_str("fn___native_fiber_drain:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    out.push_str(".L_x64_fib_drain_loop:\n");
    out.push_str("    cmpq $0, alya_fiber_runqueue_head(%rip)\n");
    out.push_str("    je .L_x64_fib_drain_done\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    jmp .L_x64_fib_drain_loop\n");
    out.push_str(".L_x64_fib_drain_done:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_count
    // =========================================================================
    out.push_str(".global fn___native_fiber_count\n");
    out.push_str("fn___native_fiber_count:\n");
    out.push_str("    mov alya_fiber_active_count(%rip), %rax\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_id
    // =========================================================================
    out.push_str(".global fn___native_fiber_id\n");
    out.push_str("fn___native_fiber_id:\n");
    out.push_str("    mov alya_fiber_current(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz .L_x64_fib_id_zero\n");
    out.push_str("    mov 0(%rax), %rax\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_fib_id_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn___native_fiber_sleep(ms)
    // Cooperative sleep: yields to other fibers while awaiting elapsed time.
    // =========================================================================
    out.push_str(".global fn___native_fiber_sleep\n");
    out.push_str("fn___native_fiber_sleep:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    push %rbx\n");
    out.push_str("    sub $40, %rsp\n");
    if is_win {
        out.push_str("    mov %rcx, %rbx\n");
    } else {
        out.push_str("    mov %rdi, %rbx\n");
    }
    out.push_str(".L_x64_fib_slp_loop:\n");
    out.push_str("    call fn___native_fiber_yield\n");
    out.push_str("    cmp $0, %rbx\n");
    out.push_str("    jle .L_x64_fib_slp_done\n");
    if is_win {
        out.push_str("    mov $1, %ecx\n");
        out.push_str("    call Sleep\n");
    } else {
        out.push_str("    mov $1000, %rdi\n");
        out.push_str(&format!("    call {}usleep\n", p));
    }
    out.push_str("    dec %rbx\n");
    out.push_str("    jmp .L_x64_fib_slp_loop\n");
    out.push_str(".L_x64_fib_slp_done:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    add $40, %rsp\n");
    out.push_str("    pop %rbx\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
