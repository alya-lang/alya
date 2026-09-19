use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_abs
    out.push_str("fn_abs:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jns .L_x86_abs_end\n");
    out.push_str("    neg %eax\n");
    out.push_str(".L_x86_abs_end:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_min
    out.push_str("fn_min:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    cmp %edx, %eax\n");
    out.push_str("    jle .L_x86_min_end\n");
    out.push_str("    mov %edx, %eax\n");
    out.push_str(".L_x86_min_end:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_max
    out.push_str("fn_max:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    cmp %edx, %eax\n");
    out.push_str("    jge .L_x86_max_end\n");
    out.push_str("    mov %edx, %eax\n");
    out.push_str(".L_x86_max_end:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_isqrt / fn_sqrt
    out.push_str(".global fn_sqrt\n");
    out.push_str("fn_sqrt:\n");
    out.push_str(".global fn_isqrt\n");
    out.push_str("fn_isqrt:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %ecx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jle .L_x86_sqrt_zero\n");
    out.push_str("    cmp $4, %ecx\n");
    out.push_str("    jl .L_x86_sqrt_one\n");
    out.push_str("    push %ebx\n");
    out.push_str("    mov %ecx, %ebx\n");
    out.push_str("    shr $1, %ebx\n");
    out.push_str(".L_x86_sqrt_loop:\n");
    out.push_str("    mov %ecx, %eax\n");
    out.push_str("    xor %edx, %edx\n");
    out.push_str("    div %ebx\n");
    out.push_str("    add %ebx, %eax\n");
    out.push_str("    shr $1, %eax\n");
    out.push_str("    cmp %ebx, %eax\n");
    out.push_str("    jge .L_x86_sqrt_done\n");
    out.push_str("    mov %eax, %ebx\n");
    out.push_str("    jmp .L_x86_sqrt_loop\n");
    out.push_str(".L_x86_sqrt_done:\n");
    out.push_str("    mov %ebx, %eax\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    jmp .L_x86_sqrt_end\n");
    out.push_str(".L_x86_sqrt_one:\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str("    jmp .L_x86_sqrt_end\n");
    out.push_str(".L_x86_sqrt_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_sqrt_end:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_pow
    out.push_str("fn_pow:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %ecx\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    js .L_x86_pow_zero\n");
    out.push_str("    mov $1, %eax\n");
    out.push_str(".L_x86_pow_loop:\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jle .L_x86_pow_end\n");
    out.push_str("    test $1, %edx\n");
    out.push_str("    jz .L_x86_pow_even\n");
    out.push_str("    imul %ecx, %eax\n");
    out.push_str(".L_x86_pow_even:\n");
    out.push_str("    imul %ecx, %ecx\n");
    out.push_str("    shr $1, %edx\n");
    out.push_str("    jmp .L_x86_pow_loop\n");
    out.push_str(".L_x86_pow_zero:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str(".L_x86_pow_end:\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // Bitwise operations
    out.push_str(".global fn_bit_and\n");
    out.push_str("fn_bit_and:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    and 12(%ebp), %eax\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_or\n");
    out.push_str("fn_bit_or:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    or 12(%ebp), %eax\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_xor\n");
    out.push_str("fn_bit_xor:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    xor 12(%ebp), %eax\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_not\n");
    out.push_str("fn_bit_not:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    not %eax\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_shl\n");
    out.push_str("fn_bit_shl:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ecx\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    shll %cl, %eax\n");
    out.push_str("    pop %ecx\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_bit_shr\n");
    out.push_str("fn_bit_shr:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ecx\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %ecx\n");
    out.push_str("    shrl %cl, %eax\n");
    out.push_str("    pop %ecx\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // PRNG
    out.push_str(".global fn_rand\n");
    out.push_str("fn_rand:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov alya_rand_state, %eax\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_rand_ok\n");
    out.push_str("    rdtsc\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jnz .L_x86_rand_ok\n");
    out.push_str("    mov $123456789, %eax\n");
    out.push_str(".L_x86_rand_ok:\n");
    out.push_str("    add $0x9e3779b9, %eax\n");
    out.push_str("    mov %eax, alya_rand_state\n");
    out.push_str("    mov %eax, %edx\n");
    out.push_str("    shr $16, %edx\n");
    out.push_str("    xor %edx, %eax\n");
    out.push_str("    imul $0x85ebca6b, %eax\n");
    out.push_str("    mov %eax, %edx\n");
    out.push_str("    shr $13, %edx\n");
    out.push_str("    xor %edx, %eax\n");
    out.push_str("    imul $0xc2b2ae35, %eax\n");
    out.push_str("    mov %eax, %edx\n");
    out.push_str("    shr $16, %edx\n");
    out.push_str("    xor %edx, %eax\n");
    out.push_str("    and $0x7fffffff, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    out.push_str(".global fn_rand_seed\n");
    out.push_str("fn_rand_seed:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov %eax, alya_rand_state\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_time
    out.push_str(".global fn_time\n");
    out.push_str("fn_time:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push $0\n");
    out.push_str("    call time\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_clock
    out.push_str(".global fn_clock\n");
    out.push_str("fn_clock:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    call clock\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // fn_clock_ms
    out.push_str(".global fn_clock_ms\n");
    out.push_str("fn_clock_ms:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    push %ebp\n");
        out.push_str("    mov %esp, %ebp\n");
        out.push_str("    call GetTickCount\n");
        out.push_str("    mov %ebp, %esp\n");
        out.push_str("    pop %ebp\n");
        out.push_str("    ret\n\n");
    } else {
        out.push_str("    push %ebp\n");
        out.push_str("    mov %esp, %ebp\n");
        out.push_str("    sub $16, %esp\n");
        out.push_str("    lea -16(%ebp), %eax\n");
        out.push_str("    push %eax\n");
        out.push_str("    push $1\n");
        out.push_str("    call clock_gettime\n");
        out.push_str("    add $8, %esp\n");
        out.push_str("    mov -12(%ebp), %eax\n");
        out.push_str("    xor %edx, %edx\n");
        out.push_str("    mov $1000000, %ecx\n");
        out.push_str("    div %ecx\n");
        out.push_str("    mov -16(%ebp), %ecx\n");
        out.push_str("    imul $1000, %ecx, %ecx\n");
        out.push_str("    add %ecx, %eax\n");
        out.push_str("    mov %ebp, %esp\n");
        out.push_str("    pop %ebp\n");
        out.push_str("    ret\n\n");
    }

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
        out.push_str("    push %ebp\n");
        out.push_str("    mov %esp, %ebp\n");
        out.push_str("    sub $24, %esp\n");
        out.push_str("    movsd %xmm0, (%esp)\n");
        out.push_str(&format!("    call {}{}\n", p, c_name));
        out.push_str("    fstpl (%esp)\n");
        out.push_str("    movsd (%esp), %xmm0\n");
        out.push_str("    mov (%esp), %eax\n");
        out.push_str("    mov %ebp, %esp\n");
        out.push_str("    pop %ebp\n");
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
        out.push_str("    push %ebp\n");
        out.push_str("    mov %esp, %ebp\n");
        out.push_str("    sub $24, %esp\n");
        out.push_str("    movsd %xmm0, (%esp)\n");
        out.push_str("    movsd %xmm1, 8(%esp)\n");
        out.push_str(&format!("    call {}{}\n", p, c_name));
        out.push_str("    fstpl (%esp)\n");
        out.push_str("    movsd (%esp), %xmm0\n");
        out.push_str("    mov (%esp), %eax\n");
        out.push_str("    mov %ebp, %esp\n");
        out.push_str("    pop %ebp\n");
        out.push_str("    ret\n\n");
    }

    emit_simd_primitives(out, os);
}

fn emit_simd_primitives(out: &mut String, _os: OperatingSystem) {
    // 1. fn_simd_f64x4_new(a, b, c, d) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_new\n");
    out.push_str("fn_simd_f64x4_new:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $12, %esp\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str("    movsd %xmm0, 0(%eax)\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str("    movsd %xmm0, 8(%eax)\n");
    out.push_str("    mov 16(%ebp), %edx\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str("    movsd %xmm0, 16(%eax)\n");
    out.push_str("    mov 20(%ebp), %edx\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str("    movsd %xmm0, 24(%eax)\n");
    out.push_str("    add $12, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 2. fn_simd_f64x4_splat(val) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_splat\n");
    out.push_str("fn_simd_f64x4_splat:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $28, %esp\n");
    out.push_str("    movsd %xmm0, -28(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movsd -28(%ebp), %xmm0\n");
    out.push_str("    xorpd %xmm1, %xmm1\n");
    out.push_str("    ucomisd %xmm1, %xmm0\n");
    out.push_str("    jne .L_x86_splat_ready\n");
    out.push_str("    jp .L_x86_splat_ready\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_splat_ready\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str(".L_x86_splat_ready:\n");
    out.push_str("    movlhps %xmm0, %xmm0\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm0, 16(%eax)\n");
    out.push_str("    add $28, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 3. fn_simd_f64x4_add(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_add\n");
    out.push_str("fn_simd_f64x4_add:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd 0(%edi), %xmm2\n");
    out.push_str("    movupd 16(%edi), %xmm3\n");
    out.push_str("    addpd %xmm2, %xmm0\n");
    out.push_str("    addpd %xmm3, %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 4. fn_simd_f64x4_sub(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_sub\n");
    out.push_str("fn_simd_f64x4_sub:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd 0(%edi), %xmm2\n");
    out.push_str("    movupd 16(%edi), %xmm3\n");
    out.push_str("    subpd %xmm2, %xmm0\n");
    out.push_str("    subpd %xmm3, %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 5. fn_simd_f64x4_mul(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_mul\n");
    out.push_str("fn_simd_f64x4_mul:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd 0(%edi), %xmm2\n");
    out.push_str("    movupd 16(%edi), %xmm3\n");
    out.push_str("    mulpd %xmm2, %xmm0\n");
    out.push_str("    mulpd %xmm3, %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 6. fn_simd_f64x4_div(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_div\n");
    out.push_str("fn_simd_f64x4_div:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd 0(%edi), %xmm2\n");
    out.push_str("    movupd 16(%edi), %xmm3\n");
    out.push_str("    divpd %xmm2, %xmm0\n");
    out.push_str("    divpd %xmm3, %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 7. fn_simd_f64x4_fma(a, b, c): (a * b) + c -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_fma\n");
    out.push_str("fn_simd_f64x4_fma:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    mov 16(%ebp), %edx\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd 0(%edi), %xmm2\n");
    out.push_str("    movupd 16(%edi), %xmm3\n");
    out.push_str("    movupd 0(%edx), %xmm4\n");
    out.push_str("    movupd 16(%edx), %xmm5\n");
    out.push_str("    mulpd %xmm2, %xmm0\n");
    out.push_str("    mulpd %xmm3, %xmm1\n");
    out.push_str("    addpd %xmm4, %xmm0\n");
    out.push_str("    addpd %xmm5, %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 8. fn_simd_f64x4_sum(a) -> float
    out.push_str(".global fn_simd_f64x4_sum\n");
    out.push_str("fn_simd_f64x4_sum:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    movupd 0(%eax), %xmm0\n");
    out.push_str("    movupd 16(%eax), %xmm1\n");
    out.push_str("    addpd %xmm1, %xmm0\n");
    out.push_str("    movhlps %xmm0, %xmm1\n");
    out.push_str("    addsd %xmm1, %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    fldl (%esp)\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 9. fn_simd_f64x4_min(a) -> float
    out.push_str(".global fn_simd_f64x4_min\n");
    out.push_str("fn_simd_f64x4_min:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    movupd 0(%eax), %xmm0\n");
    out.push_str("    movupd 16(%eax), %xmm1\n");
    out.push_str("    minpd %xmm1, %xmm0\n");
    out.push_str("    movhlps %xmm0, %xmm1\n");
    out.push_str("    minsd %xmm1, %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    fldl (%esp)\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 10. fn_simd_f64x4_max(a) -> float
    out.push_str(".global fn_simd_f64x4_max\n");
    out.push_str("fn_simd_f64x4_max:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    movupd 0(%eax), %xmm0\n");
    out.push_str("    movupd 16(%eax), %xmm1\n");
    out.push_str("    maxpd %xmm1, %xmm0\n");
    out.push_str("    movhlps %xmm0, %xmm1\n");
    out.push_str("    maxsd %xmm1, %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    fldl (%esp)\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 11. fn_simd_f64x4_get(a, idx) -> float
    out.push_str(".global fn_simd_f64x4_get\n");
    out.push_str("fn_simd_f64x4_get:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    shl $3, %edx\n");
    out.push_str("    add %edx, %eax\n");
    out.push_str("    movsd (%eax), %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    fldl (%esp)\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 12. fn_simd_f64x4_set(a, idx, val) -> ptr
    out.push_str(".global fn_simd_f64x4_set\n");
    out.push_str("fn_simd_f64x4_set:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov 12(%ebp), %edx\n");
    out.push_str("    shl $3, %edx\n");
    out.push_str("    add %edx, %eax\n");
    out.push_str("    movsd %xmm0, (%eax)\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 13. fn_simd_f64x4_load(ptr, offset) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f64x4_load\n");
    out.push_str("fn_simd_f64x4_load:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    add 12(%ebp), %esi\n");
    out.push_str("    movupd 0(%esi), %xmm0\n");
    out.push_str("    movupd 16(%esi), %xmm1\n");
    out.push_str("    movupd %xmm0, -48(%ebp)\n");
    out.push_str("    movupd %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movupd -48(%ebp), %xmm0\n");
    out.push_str("    movupd -32(%ebp), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 14. fn_simd_f64x4_store(ptr, offset, vec) -> void
    out.push_str(".global fn_simd_f64x4_store\n");
    out.push_str("fn_simd_f64x4_store:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    add 12(%ebp), %eax\n");
    out.push_str("    mov 16(%ebp), %edx\n");
    out.push_str("    movupd 0(%edx), %xmm0\n");
    out.push_str("    movupd 16(%edx), %xmm1\n");
    out.push_str("    movupd %xmm0, 0(%eax)\n");
    out.push_str("    movupd %xmm1, 16(%eax)\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 15. fn_simd_f32x8_splat(val) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f32x8_splat\n");
    out.push_str("fn_simd_f32x8_splat:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $28, %esp\n");
    out.push_str("    movsd %xmm0, -28(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movsd -28(%ebp), %xmm0\n");
    out.push_str("    xorpd %xmm1, %xmm1\n");
    out.push_str("    ucomisd %xmm1, %xmm0\n");
    out.push_str("    jne .L_x86_f32_splat_ready\n");
    out.push_str("    jp .L_x86_f32_splat_ready\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str("    jz .L_x86_f32_splat_ready\n");
    out.push_str("    cvtsi2sd %edx, %xmm0\n");
    out.push_str(".L_x86_f32_splat_ready:\n");
    out.push_str("    cvtsd2ss %xmm0, %xmm0\n");
    out.push_str("    shufps $0, %xmm0, %xmm0\n");
    out.push_str("    movups %xmm0, 0(%eax)\n");
    out.push_str("    movups %xmm0, 16(%eax)\n");
    out.push_str("    add $28, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 16. fn_simd_f32x8_add(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f32x8_add\n");
    out.push_str("fn_simd_f32x8_add:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movups 0(%esi), %xmm0\n");
    out.push_str("    movups 16(%esi), %xmm1\n");
    out.push_str("    movups 0(%edi), %xmm2\n");
    out.push_str("    movups 16(%edi), %xmm3\n");
    out.push_str("    addps %xmm2, %xmm0\n");
    out.push_str("    addps %xmm3, %xmm1\n");
    out.push_str("    movups %xmm0, -48(%ebp)\n");
    out.push_str("    movups %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movups -48(%ebp), %xmm0\n");
    out.push_str("    movups -32(%ebp), %xmm1\n");
    out.push_str("    movups %xmm0, 0(%eax)\n");
    out.push_str("    movups %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 17. fn_simd_f32x8_mul(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_f32x8_mul\n");
    out.push_str("fn_simd_f32x8_mul:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movups 0(%esi), %xmm0\n");
    out.push_str("    movups 16(%esi), %xmm1\n");
    out.push_str("    movups 0(%edi), %xmm2\n");
    out.push_str("    movups 16(%edi), %xmm3\n");
    out.push_str("    mulps %xmm2, %xmm0\n");
    out.push_str("    mulps %xmm3, %xmm1\n");
    out.push_str("    movups %xmm0, -48(%ebp)\n");
    out.push_str("    movups %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movups -48(%ebp), %xmm0\n");
    out.push_str("    movups -32(%ebp), %xmm1\n");
    out.push_str("    movups %xmm0, 0(%eax)\n");
    out.push_str("    movups %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 18. fn_simd_f32x8_sum(a) -> float
    out.push_str(".global fn_simd_f32x8_sum\n");
    out.push_str("fn_simd_f32x8_sum:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    mov 8(%ebp), %eax\n");
    out.push_str("    movups 0(%eax), %xmm0\n");
    out.push_str("    movups 16(%eax), %xmm1\n");
    out.push_str("    addps %xmm1, %xmm0\n");
    out.push_str("    movhlps %xmm0, %xmm1\n");
    out.push_str("    addps %xmm1, %xmm0\n");
    out.push_str("    movaps %xmm0, %xmm1\n");
    out.push_str("    shufps $1, %xmm1, %xmm1\n");
    out.push_str("    addss %xmm1, %xmm0\n");
    out.push_str("    cvtss2sd %xmm0, %xmm0\n");
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str("    mov (%esp), %eax\n");
    out.push_str("    fldl (%esp)\n");
    out.push_str("    add $8, %esp\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 19. fn_simd_i32x8_splat(val) -> ptr (32 bytes)
    out.push_str(".global fn_simd_i32x8_splat\n");
    out.push_str("fn_simd_i32x8_splat:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov %edx, -16(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -16(%ebp), %edx\n");
    out.push_str("    movd %edx, %xmm0\n");
    out.push_str("    pshufd $0, %xmm0, %xmm0\n");
    out.push_str("    movdqu %xmm0, 0(%eax)\n");
    out.push_str("    movdqu %xmm0, 16(%eax)\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 20. fn_simd_i32x8_add(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_i32x8_add\n");
    out.push_str("fn_simd_i32x8_add:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movdqu 0(%esi), %xmm0\n");
    out.push_str("    movdqu 16(%esi), %xmm1\n");
    out.push_str("    movdqu 0(%edi), %xmm2\n");
    out.push_str("    movdqu 16(%edi), %xmm3\n");
    out.push_str("    paddd %xmm2, %xmm0\n");
    out.push_str("    paddd %xmm3, %xmm1\n");
    out.push_str("    movdqu %xmm0, -48(%ebp)\n");
    out.push_str("    movdqu %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movdqu -48(%ebp), %xmm0\n");
    out.push_str("    movdqu -32(%ebp), %xmm1\n");
    out.push_str("    movdqu %xmm0, 0(%eax)\n");
    out.push_str("    movdqu %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 21. fn_simd_i64x4_splat(val) -> ptr (32 bytes)
    out.push_str(".global fn_simd_i64x4_splat\n");
    out.push_str("fn_simd_i64x4_splat:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $16, %esp\n");
    out.push_str("    mov 8(%ebp), %edx\n");
    out.push_str("    mov %edx, -16(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    mov -16(%ebp), %edx\n");
    out.push_str("    mov %edx, %ecx\n");
    out.push_str("    sar $31, %ecx\n");
    out.push_str("    mov %edx, 0(%eax)\n");
    out.push_str("    mov %ecx, 4(%eax)\n");
    out.push_str("    mov %edx, 8(%eax)\n");
    out.push_str("    mov %ecx, 12(%eax)\n");
    out.push_str("    mov %edx, 16(%eax)\n");
    out.push_str("    mov %ecx, 20(%eax)\n");
    out.push_str("    mov %edx, 24(%eax)\n");
    out.push_str("    mov %ecx, 28(%eax)\n");
    out.push_str("    add $16, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");

    // 22. fn_simd_i64x4_add(a, b) -> ptr (32 bytes)
    out.push_str(".global fn_simd_i64x4_add\n");
    out.push_str("fn_simd_i64x4_add:\n");
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    push %ebx\n");
    out.push_str("    push %esi\n");
    out.push_str("    push %edi\n");
    out.push_str("    sub $48, %esp\n");
    out.push_str("    mov 8(%ebp), %esi\n");
    out.push_str("    mov 12(%ebp), %edi\n");
    out.push_str("    movdqu 0(%esi), %xmm0\n");
    out.push_str("    movdqu 16(%esi), %xmm1\n");
    out.push_str("    movdqu 0(%edi), %xmm2\n");
    out.push_str("    movdqu 16(%edi), %xmm3\n");
    out.push_str("    paddq %xmm2, %xmm0\n");
    out.push_str("    paddq %xmm3, %xmm1\n");
    out.push_str("    movdqu %xmm0, -48(%ebp)\n");
    out.push_str("    movdqu %xmm1, -32(%ebp)\n");
    out.push_str("    push $32\n");
    out.push_str("    call fn_alloc\n");
    out.push_str("    add $4, %esp\n");
    out.push_str("    movdqu -48(%ebp), %xmm0\n");
    out.push_str("    movdqu -32(%ebp), %xmm1\n");
    out.push_str("    movdqu %xmm0, 0(%eax)\n");
    out.push_str("    movdqu %xmm1, 16(%eax)\n");
    out.push_str("    add $48, %esp\n");
    out.push_str("    pop %edi\n");
    out.push_str("    pop %esi\n");
    out.push_str("    pop %ebx\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n\n");
}
