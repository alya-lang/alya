use crate::codegen::target::OperatingSystem;
use super::emit_adrp_add;

#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    let is_win = matches!(os, OperatingSystem::Windows);
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let _ = (is_win, p);

    // fn_abs
    out.push_str("fn_abs:\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    b.ge .L_arm_abs_end\n");
    out.push_str("    neg x0, x0\n");
    out.push_str(".L_arm_abs_end:\n");
    out.push_str("    ret\n\n");

    // fn_min
    out.push_str("fn_min:\n");
    out.push_str("    cmp x0, x1\n");
    out.push_str("    csel x0, x0, x1, le\n");
    out.push_str("    ret\n\n");

    // fn_max
    out.push_str("fn_max:\n");
    out.push_str("    cmp x0, x1\n");
    out.push_str("    csel x0, x0, x1, ge\n");
    out.push_str("    ret\n\n");

    // fn_isqrt / fn_sqrt
    out.push_str(".global fn_sqrt\n");
    out.push_str("fn_sqrt:\n");
    out.push_str(".global fn_isqrt\n");
    out.push_str("fn_isqrt:\n");
    out.push_str("    cmp x0, #0\n");
    out.push_str("    b.le .L_arm64_sqrt_zero\n");
    out.push_str("    cmp x0, #4\n");
    out.push_str("    b.lt .L_arm64_sqrt_one\n");
    out.push_str("    mov x9, x0\n");
    out.push_str("    lsr x10, x9, #1\n");
    out.push_str(".L_arm64_sqrt_loop:\n");
    out.push_str("    sdiv x11, x9, x10\n");
    out.push_str("    add x11, x10, x11\n");
    out.push_str("    lsr x11, x11, #1\n");
    out.push_str("    cmp x11, x10\n");
    out.push_str("    b.ge .L_arm64_sqrt_done\n");
    out.push_str("    mov x10, x11\n");
    out.push_str("    b .L_arm64_sqrt_loop\n");
    out.push_str(".L_arm64_sqrt_done:\n");
    out.push_str("    mov x0, x10\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_sqrt_one:\n");
    out.push_str("    mov x0, #1\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_sqrt_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_pow
    out.push_str("fn_pow:\n");
    out.push_str("    cmp x1, #0\n");
    out.push_str("    b.lt .L_arm64_pow_zero\n");
    out.push_str("    mov x9, x0\n");
    out.push_str("    mov x10, x1\n");
    out.push_str("    mov x0, #1\n");
    out.push_str(".L_arm64_pow_loop:\n");
    out.push_str("    cmp x10, #0\n");
    out.push_str("    b.le .L_arm64_pow_end\n");
    out.push_str("    tst x10, #1\n");
    out.push_str("    b.eq .L_arm64_pow_even\n");
    out.push_str("    mul x0, x0, x9\n");
    out.push_str(".L_arm64_pow_even:\n");
    out.push_str("    mul x9, x9, x9\n");
    out.push_str("    lsr x10, x10, #1\n");
    out.push_str("    b .L_arm64_pow_loop\n");
    out.push_str(".L_arm64_pow_zero:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(".L_arm64_pow_end:\n");
    out.push_str("    ret\n\n");

    // Bitwise operations
    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_and\n");
    out.push_str("fn_bit_and:\n");
    out.push_str("    and x0, x0, x1\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_or\n");
    out.push_str("fn_bit_or:\n");
    out.push_str("    orr x0, x0, x1\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_xor\n");
    out.push_str("fn_bit_xor:\n");
    out.push_str("    eor x0, x0, x1\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_not\n");
    out.push_str("fn_bit_not:\n");
    out.push_str("    mvn x0, x0\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_shl\n");
    out.push_str("fn_bit_shl:\n");
    out.push_str("    lsl x0, x0, x1\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_bit_shr\n");
    out.push_str("fn_bit_shr:\n");
    out.push_str("    lsr x0, x0, x1\n");
    out.push_str("    ret\n\n");

    // PRNG
    out.push_str(".align 2\n");
    out.push_str(".global fn_rand\n");
    out.push_str("fn_rand:\n");
    emit_adrp_add(out, "x9", "alya_rand_state", os);
    out.push_str("    ldr x0, [x9]\n");
    out.push_str("    cbnz x0, .L_arm64_rand_ok\n");
    out.push_str("    mrs x0, cntvct_el0\n");
    out.push_str("    cbnz x0, .L_arm64_rand_ok\n");
    out.push_str("    movz x0, #0xcd15\n");
    out.push_str("    movk x0, #0x075b, lsl #16\n");
    out.push_str(".L_arm64_rand_ok:\n");
    out.push_str("    movz x10, #0x7c15\n");
    out.push_str("    movk x10, #0x7f4a, lsl #16\n");
    out.push_str("    movk x10, #0x79b9, lsl #32\n");
    out.push_str("    movk x10, #0x9e37, lsl #48\n");
    out.push_str("    add x0, x0, x10\n");
    out.push_str("    str x0, [x9]\n");
    out.push_str("    lsr x10, x0, #30\n");
    out.push_str("    eor x0, x0, x10\n");
    out.push_str("    movz x10, #0xe5b9\n");
    out.push_str("    movk x10, #0x1ce4, lsl #16\n");
    out.push_str("    movk x10, #0x476d, lsl #32\n");
    out.push_str("    movk x10, #0xbf58, lsl #48\n");
    out.push_str("    mul x0, x0, x10\n");
    out.push_str("    lsr x10, x0, #27\n");
    out.push_str("    eor x0, x0, x10\n");
    out.push_str("    movz x10, #0x11eb\n");
    out.push_str("    movk x10, #0x1331, lsl #16\n");
    out.push_str("    movk x10, #0x49bb, lsl #32\n");
    out.push_str("    movk x10, #0x94d0, lsl #48\n");
    out.push_str("    mul x0, x0, x10\n");
    out.push_str("    lsr x10, x0, #31\n");
    out.push_str("    eor x0, x0, x10\n");
    out.push_str("    lsr x0, x0, #1\n");
    out.push_str("    ret\n\n");

    out.push_str(".align 2\n");
    out.push_str(".global fn_rand_seed\n");
    out.push_str("fn_rand_seed:\n");
    emit_adrp_add(out, "x9", "alya_rand_state", os);
    out.push_str("    str x0, [x9]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // fn_time
    out.push_str(".align 2\n");
    out.push_str(".global fn_time\n");
    out.push_str("fn_time:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    bl {}time\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_clock
    out.push_str(".align 2\n");
    out.push_str(".global fn_clock\n");
    out.push_str("fn_clock:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str(&format!("    bl {}clock\n", p));
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    // fn_clock_ms
    out.push_str(".align 2\n");
    out.push_str(".global fn_clock_ms\n");
    out.push_str("fn_clock_ms:\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    stp x29, x30, [sp, #-16]!\n");
        out.push_str("    mov x29, sp\n");
        out.push_str("    bl GetTickCount64\n");
        out.push_str("    ldp x29, x30, [sp], #16\n");
        out.push_str("    ret\n\n");
    } else {
        out.push_str("    stp x29, x30, [sp, #-32]!\n");
        out.push_str("    mov x29, sp\n");
        let clock_id = if matches!(os, OperatingSystem::MacOS) { 6 } else { 1 };
        out.push_str(&format!("    mov x0, #{}\n", clock_id));
        out.push_str("    add x1, sp, #16\n");
        out.push_str(&format!("    bl {}clock_gettime\n", p));
        out.push_str("    ldr x0, [sp, #16]\n");
        out.push_str("    ldr x1, [sp, #24]\n");
        out.push_str("    mov x2, #1000\n");
        out.push_str("    mul x0, x0, x2\n");
        out.push_str("    udiv x1, x1, x2\n");
        out.push_str("    udiv x1, x1, x2\n");
        out.push_str("    add x0, x0, x1\n");
        out.push_str("    ldp x29, x30, [sp], #32\n");
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
        out.push_str(".align 2\n");
        out.push_str(&format!(".global fn_{}\n", fn_name));
        out.push_str(&format!("fn_{}:\n", fn_name));
        out.push_str("    stp x29, x30, [sp, #-16]!\n");
        out.push_str("    mov x29, sp\n");
        out.push_str("    fmov d0, x0\n");
        out.push_str(&format!("    bl {}{}\n", p, c_name));
        out.push_str("    fmov x0, d0\n");
        out.push_str("    ldp x29, x30, [sp], #16\n");
        out.push_str("    ret\n\n");
    }

    // Native libc floating-point math functions (two arguments: arg0, arg1)
    let two_arg_math = [
        ("native_atan2", "atan2"),
        ("native_fmod", "fmod"),
    ];

    for (fn_name, c_name) in two_arg_math {
        out.push_str(".align 2\n");
        out.push_str(&format!(".global fn_{}\n", fn_name));
        out.push_str(&format!("fn_{}:\n", fn_name));
        out.push_str("    stp x29, x30, [sp, #-16]!\n");
        out.push_str("    mov x29, sp\n");
        out.push_str("    fmov d0, x0\n");
        out.push_str("    fmov d1, x1\n");
        out.push_str(&format!("    bl {}{}\n", p, c_name));
        out.push_str("    fmov x0, d0\n");
        out.push_str("    ldp x29, x30, [sp], #16\n");
        out.push_str("    ret\n\n");
    }

    emit_simd_primitives(out, os);
}

fn emit_simd_primitives(out: &mut String, _os: OperatingSystem) {
    // 1. fn_simd_f64x4_new(a, b, c, d) -> x0 (ptr)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_new\n");
    out.push_str("fn_simd_f64x4_new:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    stp x0, x1, [sp, #16]\n");
    out.push_str("    stp x2, x3, [sp, #32]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp x2, x3, [sp, #16]\n");
    out.push_str("    stp x2, x3, [x0]\n");
    out.push_str("    ldp x4, x5, [sp, #32]\n");
    out.push_str("    stp x4, x5, [x0, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 2. fn_simd_f64x4_splat(val) -> x0 (ptr)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_splat\n");
    out.push_str("fn_simd_f64x4_splat:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldr x1, [sp, #16]\n");
    out.push_str("    stp x1, x1, [x0]\n");
    out.push_str("    stp x1, x1, [x0, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // 3. fn_simd_f64x4_add(a, b) -> x0 (ptr) using Neon fadd
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_add\n");
    out.push_str("fn_simd_f64x4_add:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fadd v0.2d, v0.2d, v2.2d\n");
    out.push_str("    fadd v1.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 4. fn_simd_f64x4_sub(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_sub\n");
    out.push_str("fn_simd_f64x4_sub:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fsub v0.2d, v0.2d, v2.2d\n");
    out.push_str("    fsub v1.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 5. fn_simd_f64x4_mul(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_mul\n");
    out.push_str("fn_simd_f64x4_mul:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fmul v0.2d, v0.2d, v2.2d\n");
    out.push_str("    fmul v1.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 6. fn_simd_f64x4_div(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_div\n");
    out.push_str("fn_simd_f64x4_div:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fdiv v0.2d, v0.2d, v2.2d\n");
    out.push_str("    fdiv v1.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 7. fn_simd_f64x4_fma(a, b, c): (a * b) + c using Neon fmla
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_fma\n");
    out.push_str("fn_simd_f64x4_fma:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    ldp q4, q5, [x2]\n");
    out.push_str("    fmla v4.2d, v0.2d, v2.2d\n");
    out.push_str("    fmla v5.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q4, q5, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q4, q5, [sp, #16]\n");
    out.push_str("    stp q4, q5, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 8. fn_simd_f64x4_sum(a) -> float in x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_sum\n");
    out.push_str("fn_simd_f64x4_sum:\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    faddp d0, v0.2d\n");
    out.push_str("    faddp d1, v1.2d\n");
    out.push_str("    fadd d0, d0, d1\n");
    out.push_str("    fmov x0, d0\n");
    out.push_str("    ret\n\n");

    // 9. fn_simd_f64x4_min(a) -> float in x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_min\n");
    out.push_str("fn_simd_f64x4_min:\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    fminp d0, v0.2d\n");
    out.push_str("    fminp d1, v1.2d\n");
    out.push_str("    fmin d0, d0, d1\n");
    out.push_str("    fmov x0, d0\n");
    out.push_str("    ret\n\n");

    // 10. fn_simd_f64x4_max(a) -> float in x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_max\n");
    out.push_str("fn_simd_f64x4_max:\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    fmaxp d0, v0.2d\n");
    out.push_str("    fmaxp d1, v1.2d\n");
    out.push_str("    fmax d0, d0, d1\n");
    out.push_str("    fmov x0, d0\n");
    out.push_str("    ret\n\n");

    // 11. fn_simd_f64x4_get(a, idx) -> x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_get\n");
    out.push_str("fn_simd_f64x4_get:\n");
    out.push_str("    ldr x0, [x0, x1, lsl #3]\n");
    out.push_str("    ret\n\n");

    // 12. fn_simd_f64x4_set(a, idx, val) -> x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_set\n");
    out.push_str("fn_simd_f64x4_set:\n");
    out.push_str("    str x2, [x0, x1, lsl #3]\n");
    out.push_str("    ret\n\n");

    // 13. fn_simd_f64x4_load(ptr, offset) -> x0
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_load\n");
    out.push_str("fn_simd_f64x4_load:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 14. fn_simd_f64x4_store(ptr, offset, vec)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f64x4_store\n");
    out.push_str("fn_simd_f64x4_store:\n");
    out.push_str("    add x0, x0, x1\n");
    out.push_str("    ldp q0, q1, [x2]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // 15. fn_simd_f32x8_splat(val)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f32x8_splat\n");
    out.push_str("fn_simd_f32x8_splat:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldr w1, [sp, #16]\n");
    out.push_str("    dup v0.4s, w1\n");
    out.push_str("    stp q0, q0, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // 16. fn_simd_f32x8_add(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f32x8_add\n");
    out.push_str("fn_simd_f32x8_add:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fadd v0.4s, v0.4s, v2.4s\n");
    out.push_str("    fadd v1.4s, v1.4s, v3.4s\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 17. fn_simd_f32x8_mul(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f32x8_mul\n");
    out.push_str("fn_simd_f32x8_mul:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    fmul v0.4s, v0.4s, v2.4s\n");
    out.push_str("    fmul v1.4s, v1.4s, v3.4s\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 18. fn_simd_f32x8_sum(a) -> float
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_f32x8_sum\n");
    out.push_str("fn_simd_f32x8_sum:\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    fadd v0.4s, v0.4s, v1.4s\n");
    out.push_str("    faddp v0.4s, v0.4s, v0.4s\n");
    out.push_str("    faddp s0, v0.2s\n");
    out.push_str("    fcvt d0, s0\n");
    out.push_str("    fmov x0, d0\n");
    out.push_str("    ret\n\n");

    // 19. fn_simd_i32x8_splat(val)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_i32x8_splat\n");
    out.push_str("fn_simd_i32x8_splat:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str w0, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldr w1, [sp, #16]\n");
    out.push_str("    dup v0.4s, w1\n");
    out.push_str("    stp q0, q0, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // 20. fn_simd_i32x8_add(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_i32x8_add\n");
    out.push_str("fn_simd_i32x8_add:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    add v0.4s, v0.4s, v2.4s\n");
    out.push_str("    add v1.4s, v1.4s, v3.4s\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");

    // 21. fn_simd_i64x4_splat(val)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_i64x4_splat\n");
    out.push_str("fn_simd_i64x4_splat:\n");
    out.push_str("    stp x29, x30, [sp, #-32]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    str x0, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldr x1, [sp, #16]\n");
    out.push_str("    stp x1, x1, [x0]\n");
    out.push_str("    stp x1, x1, [x0, #16]\n");
    out.push_str("    ldp x29, x30, [sp], #32\n");
    out.push_str("    ret\n\n");

    // 22. fn_simd_i64x4_add(a, b)
    out.push_str(".align 2\n");
    out.push_str(".global fn_simd_i64x4_add\n");
    out.push_str("fn_simd_i64x4_add:\n");
    out.push_str("    stp x29, x30, [sp, #-48]!\n");
    out.push_str("    mov x29, sp\n");
    out.push_str("    ldp q0, q1, [x0]\n");
    out.push_str("    ldp q2, q3, [x1]\n");
    out.push_str("    add v0.2d, v0.2d, v2.2d\n");
    out.push_str("    add v1.2d, v1.2d, v3.2d\n");
    out.push_str("    stp q0, q1, [sp, #16]\n");
    out.push_str("    mov x0, #32\n");
    out.push_str("    bl fn_alloc\n");
    out.push_str("    ldp q0, q1, [sp, #16]\n");
    out.push_str("    stp q0, q1, [x0]\n");
    out.push_str("    ldp x29, x30, [sp], #48\n");
    out.push_str("    ret\n\n");
}
