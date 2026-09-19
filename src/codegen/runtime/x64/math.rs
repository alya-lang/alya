use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_abs
    out.push_str("fn_abs:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jns .L_x64_abs_end\n");
    out.push_str("    neg %rax\n");
    out.push_str(".L_x64_abs_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_min
    out.push_str("fn_min:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    cmp %rdx, %rax\n");
        out.push_str("    jle .L_x64_min_end\n");
        out.push_str("    mov %rdx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    cmp %rsi, %rax\n");
        out.push_str("    jle .L_x64_min_end\n");
        out.push_str("    mov %rsi, %rax\n");
    }
    out.push_str(".L_x64_min_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_max
    out.push_str("fn_max:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    cmp %rdx, %rax\n");
        out.push_str("    jge .L_x64_max_end\n");
        out.push_str("    mov %rdx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    cmp %rsi, %rax\n");
        out.push_str("    jge .L_x64_max_end\n");
        out.push_str("    mov %rsi, %rax\n");
    }
    out.push_str(".L_x64_max_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_isqrt / fn_sqrt
    out.push_str(".global fn_sqrt\n");
    out.push_str("fn_sqrt:\n");
    out.push_str(".global fn_isqrt\n");
    out.push_str("fn_isqrt:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r8\n");
    } else {
        out.push_str("    mov %rdi, %r8\n");
    }
    out.push_str("    test %r8, %r8\n");
    out.push_str("    jle .L_x64_sqrt_zero\n");
    out.push_str("    cmp $4, %r8\n");
    out.push_str("    jl .L_x64_sqrt_one\n");
    out.push_str("    mov %r8, %r9\n");
    out.push_str("    shr $1, %r9\n");
    out.push_str(".L_x64_sqrt_loop:\n");
    out.push_str("    mov %r8, %rax\n");
    out.push_str("    xor %rdx, %rdx\n");
    out.push_str("    div %r9\n");
    out.push_str("    add %r9, %rax\n");
    out.push_str("    shr $1, %rax\n");
    out.push_str("    cmp %r9, %rax\n");
    out.push_str("    jge .L_x64_sqrt_done\n");
    out.push_str("    mov %rax, %r9\n");
    out.push_str("    jmp .L_x64_sqrt_loop\n");
    out.push_str(".L_x64_sqrt_done:\n");
    out.push_str("    mov %r9, %rax\n");
    out.push_str("    jmp .L_x64_sqrt_end\n");
    out.push_str(".L_x64_sqrt_one:\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str("    jmp .L_x64_sqrt_end\n");
    out.push_str(".L_x64_sqrt_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_sqrt_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_pow
    out.push_str("fn_pow:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %r8\n");
        out.push_str("    mov %rdx, %r9\n");
    } else {
        out.push_str("    mov %rdi, %r8\n");
        out.push_str("    mov %rsi, %r9\n");
    }
    out.push_str("    test %r9, %r9\n");
    out.push_str("    js .L_x64_pow_zero\n");
    out.push_str("    mov $1, %rax\n");
    out.push_str(".L_x64_pow_loop:\n");
    out.push_str("    test %r9, %r9\n");
    out.push_str("    jle .L_x64_pow_end\n");
    out.push_str("    test $1, %r9\n");
    out.push_str("    jz .L_x64_pow_even\n");
    out.push_str("    imul %r8, %rax\n");
    out.push_str(".L_x64_pow_even:\n");
    out.push_str("    imul %r8, %r8\n");
    out.push_str("    shr $1, %r9\n");
    out.push_str("    jmp .L_x64_pow_loop\n");
    out.push_str(".L_x64_pow_zero:\n");
    out.push_str("    xor %rax, %rax\n");
    out.push_str(".L_x64_pow_end:\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");


    // Bitwise operations
    out.push_str(".global fn_bit_and\n");
    out.push_str("fn_bit_and:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    and %rdx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    and %rsi, %rax\n");
    }
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_or\n");
    out.push_str("fn_bit_or:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    or %rdx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    or %rsi, %rax\n");
    }
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_xor\n");
    out.push_str("fn_bit_xor:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    xor %rdx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    xor %rsi, %rax\n");
    }
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_not\n");
    out.push_str("fn_bit_not:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    not %rax\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_shl\n");
    out.push_str("fn_bit_shl:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    mov %rdx, %rcx\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    mov %rsi, %rcx\n");
    }
    out.push_str("    shl %cl, %rax\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_shr\n");
    out.push_str("fn_bit_shr:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, %rax\n");
        out.push_str("    mov %rdx, %rcx\n");
    } else {
        out.push_str("    mov %rdi, %rax\n");
        out.push_str("    mov %rsi, %rcx\n");
    }
    out.push_str("    shr %cl, %rax\n");
    out.push_str("    ret\n\n");

    // PRNG
    out.push_str(".global fn_rand\n");
    out.push_str("fn_rand:\n");
    out.push_str("    mov alya_rand_state(%rip), %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_rand_ok\n");
    out.push_str("    rdtsc\n");
    out.push_str("    shl $32, %rdx\n");
    out.push_str("    or %rdx, %rax\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jnz .L_x64_rand_ok\n");
    out.push_str("    mov $123456789, %rax\n");
    out.push_str(".L_x64_rand_ok:\n");
    out.push_str("    mov $0x9e3779b97f4a7c15, %rcx\n");
    out.push_str("    add %rcx, %rax\n");
    out.push_str("    mov %rax, alya_rand_state(%rip)\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    shr $30, %rdx\n");
    out.push_str("    xor %rdx, %rax\n");
    out.push_str("    mov $0xbf58476d1ce4e5b9, %rdx\n");
    out.push_str("    imul %rdx, %rax\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    shr $27, %rdx\n");
    out.push_str("    xor %rdx, %rax\n");
    out.push_str("    mov $0x94d049bb133111eb, %rdx\n");
    out.push_str("    imul %rdx, %rax\n");
    out.push_str("    mov %rax, %rdx\n");
    out.push_str("    shr $31, %rdx\n");
    out.push_str("    xor %rdx, %rax\n");
    out.push_str("    btr $63, %rax\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_rand_seed\n");
    out.push_str("fn_rand_seed:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rcx, alya_rand_state(%rip)\n");
    } else {
        out.push_str("    mov %rdi, alya_rand_state(%rip)\n");
    }
    out.push_str("    xor %rax, %rax\n");
    out.push_str("    ret\n\n");

    // fn_time
    out.push_str(".global fn_time\n");
    out.push_str("fn_time:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    out.push_str("    xor %rcx, %rcx\n");
    out.push_str("    xor %rdi, %rdi\n");
    out.push_str(&format!("    call {}time\n", p));
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_clock
    out.push_str(".global fn_clock\n");
    out.push_str("fn_clock:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $32, %rsp\n");
    out.push_str(&format!("    call {}clock\n", p));
    out.push_str("    add $32, %rsp\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // fn_clock_ms
    out.push_str(".global fn_clock_ms\n");
    out.push_str("fn_clock_ms:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call GetTickCount64\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        out.push_str("    sub $16, %rsp\n");
        let clock_id = if matches!(os, OperatingSystem::MacOS) { 6 } else { 1 };
        out.push_str(&format!("    mov ${}, %rdi\n", clock_id));
        out.push_str("    lea -16(%rbp), %rsi\n");
        out.push_str(&format!("    call {}clock_gettime\n", p));
        out.push_str("    mov -8(%rbp), %rax\n");
        out.push_str("    xor %rdx, %rdx\n");
        out.push_str("    mov $1000000, %rcx\n");
        out.push_str("    div %rcx\n");
        out.push_str("    mov -16(%rbp), %rcx\n");
        out.push_str("    imul $1000, %rcx, %rcx\n");
        out.push_str("    add %rcx, %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // Native libc floating-point math functions (single argument)
    let single_arg_math = [
        ("native_sin", "sin"),
        ("native_cos", "cos"),
        ("native_tan", "tan"),
        ("native_asin", "asin"),
        ("native_acos", "acos"),
        ("native_atan", "atan"),
        ("native_sinh", "sinh"),
        ("native_cosh", "cosh"),
        ("native_tanh", "tanh"),
        ("native_log", "log"),
        ("native_log2", "log2"),
        ("native_log10", "log10"),
        ("native_exp", "exp"),
        ("native_sqrt", "sqrt"),
        ("native_ceil", "ceil"),
        ("native_floor", "floor"),
    ];

    for (fn_name, c_name) in single_arg_math {
        out.push_str(&format!(".global fn_{}\n", fn_name));
        out.push_str(&format!("fn_{}:\n", fn_name));
        out.push_str("    push %rbp\n");
        out.push_str("    mov %rsp, %rbp\n");
        out.push_str("    sub $32, %rsp\n");
        if is_win {
            out.push_str("    movq %rcx, %xmm0\n");
        } else {
            out.push_str("    movq %rdi, %xmm0\n");
        }
        out.push_str(&format!("    call {}{}\n", p, c_name));
        out.push_str("    movq %xmm0, %rax\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    mov %rbp, %rsp\n");
        out.push_str("    pop %rbp\n");
        out.push_str("    ret\n\n");
    }

    // Native libc floating-point math functions (two arguments: arg0, arg1)
    let two_arg_math = [
        ("native_atan2", "atan2"),
        ("native_fmod", "fmod"),
    ];

    for (fn_name, c_name) in two_arg_math {
        out.push_str(&format!(".global fn_{}\n", fn_name));
        out.push_str(&format!("fn_{}:\n", fn_name));
        out.push_str("    push %rbp\n");
        out.push_str("    mov %rsp, %rbp\n");
        out.push_str("    sub $32, %rsp\n");
        if is_win {
            out.push_str("    movq %rcx, %xmm0\n");
            out.push_str("    movq %rdx, %xmm1\n");
        } else {
            out.push_str("    movq %rdi, %xmm0\n");
            out.push_str("    movq %rsi, %xmm1\n");
        }
        out.push_str(&format!("    call {}{}\n", p, c_name));
        out.push_str("    movq %xmm0, %rax\n");
        out.push_str("    add $32, %rsp\n");
        out.push_str("    mov %rbp, %rsp\n");
        out.push_str("    pop %rbp\n");
        out.push_str("    ret\n\n");
    }

    emit_simd_primitives(out, os);
}

fn emit_simd_primitives(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);

    // 1. fn_simd_f64x4_new(a, b, c, d) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_new\n");
    out.push_str("fn_simd_f64x4_new:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    movq %rcx, 0(%rsp)\n");
        out.push_str("    movq %rdx, 8(%rsp)\n");
        out.push_str("    movq %r8, 16(%rsp)\n");
        out.push_str("    movq %r9, 24(%rsp)\n");
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    movq %rdi, 0(%rsp)\n");
        out.push_str("    movq %rsi, 8(%rsp)\n");
        out.push_str("    movq %rdx, 16(%rsp)\n");
        out.push_str("    movq %rcx, 24(%rsp)\n");
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    movq 0(%rsp), %r10\n");
    out.push_str("    movq %r10, 0(%rax)\n");
    out.push_str("    movq 8(%rsp), %r10\n");
    out.push_str("    movq %r10, 8(%rax)\n");
    out.push_str("    movq 16(%rsp), %r10\n");
    out.push_str("    movq %r10, 16(%rax)\n");
    out.push_str("    movq 24(%rsp), %r10\n");
    out.push_str("    movq %r10, 24(%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 2. fn_simd_f64x4_splat(val) -> ptr
    out.push_str(".global fn_simd_f64x4_splat\n");
    out.push_str("fn_simd_f64x4_splat:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    movq %rcx, 0(%rsp)\n");
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    movq %rdi, 0(%rsp)\n");
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    movq 0(%rsp), %r10\n");
    out.push_str("    movq %r10, 0(%rax)\n");
    out.push_str("    movq %r10, 8(%rax)\n");
    out.push_str("    movq %r10, 16(%rax)\n");
    out.push_str("    movq %r10, 24(%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 3. fn_simd_f64x4_add(a, b) -> ptr
    out.push_str(".global fn_simd_f64x4_add\n");
    out.push_str("fn_simd_f64x4_add:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
        out.push_str("    vmovupd (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
        out.push_str("    vmovupd (%rsi), %ymm1\n");
    }
    out.push_str("    vaddpd %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 4. fn_simd_f64x4_sub(a, b) -> ptr
    out.push_str(".global fn_simd_f64x4_sub\n");
    out.push_str("fn_simd_f64x4_sub:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
        out.push_str("    vmovupd (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
        out.push_str("    vmovupd (%rsi), %ymm1\n");
    }
    out.push_str("    vsubpd %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 5. fn_simd_f64x4_mul(a, b) -> ptr
    out.push_str(".global fn_simd_f64x4_mul\n");
    out.push_str("fn_simd_f64x4_mul:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
        out.push_str("    vmovupd (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
        out.push_str("    vmovupd (%rsi), %ymm1\n");
    }
    out.push_str("    vmulpd %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 6. fn_simd_f64x4_div(a, b) -> ptr
    out.push_str(".global fn_simd_f64x4_div\n");
    out.push_str("fn_simd_f64x4_div:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
        out.push_str("    vmovupd (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
        out.push_str("    vmovupd (%rsi), %ymm1\n");
    }
    out.push_str("    vdivpd %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 7. fn_simd_f64x4_fma(a, b, c) -> ptr: (a * b) + c
    out.push_str(".global fn_simd_f64x4_fma\n");
    out.push_str("fn_simd_f64x4_fma:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
        out.push_str("    vmovupd (%rdx), %ymm1\n");
        out.push_str("    vmovupd (%r8), %ymm2\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
        out.push_str("    vmovupd (%rsi), %ymm1\n");
        out.push_str("    vmovupd (%rdx), %ymm2\n");
    }
    out.push_str("    vfmadd213pd %ymm2, %ymm1, %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 8. fn_simd_f64x4_sum(a) -> float (horizontal sum)
    out.push_str(".global fn_simd_f64x4_sum\n");
    out.push_str("fn_simd_f64x4_sum:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
    }
    out.push_str("    vextractf128 $1, %ymm0, %xmm1\n");
    out.push_str("    vaddpd %xmm1, %xmm0, %xmm0\n");
    out.push_str("    vhaddpd %xmm0, %xmm0, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 9. fn_simd_f64x4_min(a) -> float
    out.push_str(".global fn_simd_f64x4_min\n");
    out.push_str("fn_simd_f64x4_min:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
    }
    out.push_str("    vextractf128 $1, %ymm0, %xmm1\n");
    out.push_str("    vminpd %xmm1, %xmm0, %xmm0\n");
    out.push_str("    vpermilpd $1, %xmm0, %xmm1\n");
    out.push_str("    vminpd %xmm1, %xmm0, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 10. fn_simd_f64x4_max(a) -> float
    out.push_str(".global fn_simd_f64x4_max\n");
    out.push_str("fn_simd_f64x4_max:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    vmovupd (%rcx), %ymm0\n");
    } else {
        out.push_str("    vmovupd (%rdi), %ymm0\n");
    }
    out.push_str("    vextractf128 $1, %ymm0, %xmm1\n");
    out.push_str("    vmaxpd %xmm1, %xmm0, %xmm0\n");
    out.push_str("    vpermilpd $1, %xmm0, %xmm1\n");
    out.push_str("    vmaxpd %xmm1, %xmm0, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 11. fn_simd_f64x4_get(a, idx) -> float
    out.push_str(".global fn_simd_f64x4_get\n");
    out.push_str("fn_simd_f64x4_get:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    movq (%rcx, %rdx, 8), %rax\n");
    } else {
        out.push_str("    movq (%rdi, %rsi, 8), %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 12. fn_simd_f64x4_set(a, idx, val) -> ptr
    out.push_str(".global fn_simd_f64x4_set\n");
    out.push_str("fn_simd_f64x4_set:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    movq %r8, (%rcx, %rdx, 8)\n");
        out.push_str("    mov %rcx, %rax\n");
    } else {
        out.push_str("    movq %rdx, (%rdi, %rsi, 8)\n");
        out.push_str("    mov %rdi, %rax\n");
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 13. fn_simd_f64x4_load(ptr, offset) -> ptr
    out.push_str(".global fn_simd_f64x4_load\n");
    out.push_str("fn_simd_f64x4_load:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    lea (%rcx, %rdx), %r10\n");
    } else {
        out.push_str("    lea (%rdi, %rsi), %r10\n");
    }
    out.push_str("    vmovupd (%r10), %ymm0\n");
    out.push_str("    vmovupd %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovupd 0(%rsp), %ymm0\n");
    out.push_str("    vmovupd %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 14. fn_simd_f64x4_store(ptr, offset, vec) -> void
    out.push_str(".global fn_simd_f64x4_store\n");
    out.push_str("fn_simd_f64x4_store:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    lea (%rcx, %rdx), %r10\n");
        out.push_str("    vmovupd (%r8), %ymm0\n");
    } else {
        out.push_str("    lea (%rdi, %rsi), %r10\n");
        out.push_str("    vmovupd (%rdx), %ymm0\n");
    }
    out.push_str("    vmovupd %ymm0, (%r10)\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // --- f32x8 Primitives ---
    // 15. fn_simd_f32x8_splat(val)
    out.push_str(".global fn_simd_f32x8_splat\n");
    out.push_str("fn_simd_f32x8_splat:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    movq %rcx, 0(%rsp)\n");
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    movq %rdi, 0(%rsp)\n");
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    movq 0(%rsp), %r10\n");
    out.push_str("    mov %r10d, 0(%rax)\n");
    out.push_str("    mov %r10d, 4(%rax)\n");
    out.push_str("    mov %r10d, 8(%rax)\n");
    out.push_str("    mov %r10d, 12(%rax)\n");
    out.push_str("    mov %r10d, 16(%rax)\n");
    out.push_str("    mov %r10d, 20(%rax)\n");
    out.push_str("    mov %r10d, 24(%rax)\n");
    out.push_str("    mov %r10d, 28(%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 16. fn_simd_f32x8_add(a, b) -> vaddps
    out.push_str(".global fn_simd_f32x8_add\n");
    out.push_str("fn_simd_f32x8_add:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovups (%rcx), %ymm0\n");
        out.push_str("    vmovups (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovups (%rdi), %ymm0\n");
        out.push_str("    vmovups (%rsi), %ymm1\n");
    }
    out.push_str("    vaddps %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovups %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovups 0(%rsp), %ymm0\n");
    out.push_str("    vmovups %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 17. fn_simd_f32x8_mul(a, b) -> vmulps
    out.push_str(".global fn_simd_f32x8_mul\n");
    out.push_str("fn_simd_f32x8_mul:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovups (%rcx), %ymm0\n");
        out.push_str("    vmovups (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovups (%rdi), %ymm0\n");
        out.push_str("    vmovups (%rsi), %ymm1\n");
    }
    out.push_str("    vmulps %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovups %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovups 0(%rsp), %ymm0\n");
    out.push_str("    vmovups %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 18. fn_simd_f32x8_sum(a) -> float
    out.push_str(".global fn_simd_f32x8_sum\n");
    out.push_str("fn_simd_f32x8_sum:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if is_win {
        out.push_str("    vmovups (%rcx), %ymm0\n");
    } else {
        out.push_str("    vmovups (%rdi), %ymm0\n");
    }
    out.push_str("    vextractf128 $1, %ymm0, %xmm1\n");
    out.push_str("    vaddps %xmm1, %xmm0, %xmm0\n");
    out.push_str("    vhaddps %xmm0, %xmm0, %xmm0\n");
    out.push_str("    vhaddps %xmm0, %xmm0, %xmm0\n");
    out.push_str("    cvtss2sd %xmm0, %xmm0\n");
    out.push_str("    movq %xmm0, %rax\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // --- i32x8 Primitives ---
    // 19. fn_simd_i32x8_splat(val)
    out.push_str(".global fn_simd_i32x8_splat\n");
    out.push_str("fn_simd_i32x8_splat:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    movq %rcx, 0(%rsp)\n");
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    movq %rdi, 0(%rsp)\n");
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    movq 0(%rsp), %r10\n");
    out.push_str("    mov %r10d, 0(%rax)\n");
    out.push_str("    mov %r10d, 4(%rax)\n");
    out.push_str("    mov %r10d, 8(%rax)\n");
    out.push_str("    mov %r10d, 12(%rax)\n");
    out.push_str("    mov %r10d, 16(%rax)\n");
    out.push_str("    mov %r10d, 20(%rax)\n");
    out.push_str("    mov %r10d, 24(%rax)\n");
    out.push_str("    mov %r10d, 28(%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 20. fn_simd_i32x8_add(a, b) -> vpaddd
    out.push_str(".global fn_simd_i32x8_add\n");
    out.push_str("fn_simd_i32x8_add:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovdqu (%rcx), %ymm0\n");
        out.push_str("    vmovdqu (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovdqu (%rdi), %ymm0\n");
        out.push_str("    vmovdqu (%rsi), %ymm1\n");
    }
    out.push_str("    vpaddd %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovdqu %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovdqu 0(%rsp), %ymm0\n");
    out.push_str("    vmovdqu %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // --- i64x4 Primitives ---
    // 21. fn_simd_i64x4_splat(val)
    out.push_str(".global fn_simd_i64x4_splat\n");
    out.push_str("fn_simd_i64x4_splat:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $48, %rsp\n");
    if is_win {
        out.push_str("    movq %rcx, 0(%rsp)\n");
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    movq %rdi, 0(%rsp)\n");
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    movq 0(%rsp), %r10\n");
    out.push_str("    movq %r10, 0(%rax)\n");
    out.push_str("    movq %r10, 8(%rax)\n");
    out.push_str("    movq %r10, 16(%rax)\n");
    out.push_str("    movq %r10, 24(%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    // 22. fn_simd_i64x4_add(a, b) -> vpaddq
    out.push_str(".global fn_simd_i64x4_add\n");
    out.push_str("fn_simd_i64x4_add:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    out.push_str("    sub $64, %rsp\n");
    if is_win {
        out.push_str("    vmovdqu (%rcx), %ymm0\n");
        out.push_str("    vmovdqu (%rdx), %ymm1\n");
    } else {
        out.push_str("    vmovdqu (%rdi), %ymm0\n");
        out.push_str("    vmovdqu (%rsi), %ymm1\n");
    }
    out.push_str("    vpaddq %ymm1, %ymm0, %ymm0\n");
    out.push_str("    vmovdqu %ymm0, 0(%rsp)\n");
    if is_win {
        out.push_str("    mov $32, %rcx\n");
    } else {
        out.push_str("    mov $32, %rdi\n");
    }
    out.push_str("    call fn_alloc\n");
    out.push_str("    vmovdqu 0(%rsp), %ymm0\n");
    out.push_str("    vmovdqu %ymm0, (%rax)\n");
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");
}
