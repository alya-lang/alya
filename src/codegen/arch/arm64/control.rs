use super::loads::{emit_arm64_load_x29_offset, emit_arm64_store_x29_offset};
use crate::codegen::target::OperatingSystem;

pub fn emit_jump_if_zero(out: &mut String, label: &str) {
    out.push_str(&format!("    cbz x0, {}\n", label));
}

pub fn emit_jump_if_not_zero(out: &mut String, label: &str) {
    out.push_str(&format!("    cbnz x0, {}\n", label));
}

pub fn emit_jump(out: &mut String, label: &str) {
    out.push_str(&format!("    b {}\n", label));
}

pub fn emit_compare_and_jump_if_greater(out: &mut String, label: &str, unsigned: bool) {
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str("    cmp x1, x0\n");
    if unsigned {
        out.push_str(&format!("    b.hi {}\n", label));
    } else {
        out.push_str(&format!("    b.gt {}\n", label));
    }
}

pub fn emit_compare_and_jump_if_greater_or_equal(out: &mut String, label: &str, unsigned: bool) {
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str("    cmp x1, x0\n");
    if unsigned {
        out.push_str(&format!("    b.hs {}\n", label));
    } else {
        out.push_str(&format!("    b.ge {}\n", label));
    }
}

pub fn emit_increment_var(
    out: &mut String,
    var_offset: i32,
    _stack_offset: i32,
    start_label: &str,
) {
    emit_arm64_load_x29_offset(out, "x0", var_offset, "x9");
    out.push_str("    add x0, x0, #1\n");
    emit_arm64_store_x29_offset(out, "x0", var_offset, "x9");
    out.push_str(&format!("    b {}\n", start_label));
}

pub fn emit_function_prologue(out: &mut String, name: &str, _os: OperatingSystem) {
    out.push_str(&format!("\n.globl fn_{}\n", name));
    out.push_str(&format!("fn_{}:\n", name));
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n\n");
}

/// Emits a global alias label for `@export("name")` (Chapter 18 §1.2).
/// Must precede the function prologue so both labels share one address.
pub fn emit_export_alias(out: &mut String, alias: &str) {
    out.push_str(&format!("\n.globl {}\n", alias));
    out.push_str(&format!("{}:\n", alias));
}

pub fn emit_function_param_push(out: &mut String, param_idx: usize, stack_offset: &mut i32) {
    *stack_offset += 16;
    if param_idx < 8 {
        let reg = match param_idx {
            0 => "x0",
            1 => "x1",
            2 => "x2",
            3 => "x3",
            4 => "x4",
            5 => "x5",
            6 => "x6",
            7 => "x7",
            _ => unreachable!(),
        };
        out.push_str(&format!("    str {}, [sp, #-16]!\n", reg));
    } else {
        let src_offset = 16 + (param_idx - 8) * 8;
        out.push_str(&format!("    ldr x9, [x29, #{}]\n", src_offset));
        out.push_str("    str x9, [sp, #-16]!\n");
    }
}

pub fn emit_function_epilogue(out: &mut String) {
    out.push_str("    mov sp, x29\n");
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");
}

pub fn emit_call_target(out: &mut String, target: &str, args_count: usize) {
    if args_count <= 8 {
        for i in (0..args_count).rev() {
            let reg = match i {
                0 => "x0",
                1 => "x1",
                2 => "x2",
                3 => "x3",
                4 => "x4",
                5 => "x5",
                6 => "x6",
                7 => "x7",
                _ => unreachable!(),
            };
            out.push_str(&format!("    ldr {}, [sp], #16\n", reg));
        }
        let call_insn = if target.starts_with('x') { "blr" } else { "bl" };
        out.push_str(&format!("    {} {}\n", call_insn, target));
    } else {
        let extra_args = args_count - 8;
        let needed = extra_args as i32 * 8;
        let total_alloc = if needed % 16 == 0 { needed } else { needed + 8 };

        for i in 0..8 {
            let offset = (args_count - 1 - i) * 16;
            out.push_str(&format!("    ldr x{}, [sp, #{}]\n", i, offset));
        }

        out.push_str(&format!("    sub sp, sp, #{}\n", total_alloc));

        for k in 8..args_count {
            let src_off = total_alloc + ((args_count - 1 - k) * 16) as i32;
            let dst_off = ((k - 8) * 8) as i32;
            out.push_str(&format!("    ldr x9, [sp, #{}]\n", src_off));
            out.push_str(&format!("    str x9, [sp, #{}]\n", dst_off));
        }

        let call_insn = if target.starts_with('x') { "blr" } else { "bl" };
        out.push_str(&format!("    {} {}\n", call_insn, target));

        let total_restore = total_alloc + args_count as i32 * 16;
        out.push_str(&format!("    add sp, sp, #{}\n", total_restore));
    }
}

pub fn emit_function_call(out: &mut String, name: &str, args_count: usize) {
    emit_call_target(out, &format!("fn_{}", name), args_count);
}

pub fn emit_indirect_function_call(out: &mut String, var_offset: i32, args_count: usize) {
    emit_arm64_load_x29_offset(out, "x16", var_offset, "x9");
    emit_call_target(out, "x16", args_count);
}

/// Guarded indirect call (alya-lang/alya#154): arm64 mirror of the x64
/// helper above. x16 carries the callee, x9/x10 are scratch.
pub fn emit_guarded_indirect_function_call(
    out: &mut String,
    var_offset: i32,
    args_count: usize,
    os: OperatingSystem,
    ro_label: &str,
    call_label: &str,
) {
    use super::emit_adrp_add;
    emit_arm64_load_x29_offset(out, "x16", var_offset, "x9");
    out.push_str("    movz x9, #1, lsl #16\n");
    out.push_str("    cmp x16, x9\n");
    out.push_str("    b.lo alya_error_not_callable\n");
    emit_adrp_add(out, "x9", "alya_str_buf", os);
    out.push_str("    cmp x16, x9\n");
    out.push_str(&format!("    b.lo {}\n", ro_label));
    out.push_str("    movz x10, #1024, lsl #16\n");
    out.push_str("    add x10, x9, x10\n");
    out.push_str("    cmp x16, x10\n");
    out.push_str("    b.lo alya_error_not_callable\n");
    out.push_str(&format!("{}:\n", ro_label));
    emit_adrp_add(out, "x9", "alya_rodata_start", os);
    out.push_str("    cmp x16, x9\n");
    out.push_str(&format!("    b.lo {}\n", call_label));
    emit_adrp_add(out, "x10", "alya_rodata_end", os);
    out.push_str("    cmp x16, x10\n");
    out.push_str("    b.lo alya_error_not_callable\n");
    out.push_str(&format!("{}:\n", call_label));
    // alya-lang/alya#100: closure dispatch (mirror of x64): a slot
    // holding a closure object calls its code with the stored chain
    // appended (pushed first so it lands at index N).
    out.push_str("    ldur w9, [x16, #-16]\n");
    out.push_str("    movz w10, #0x0005\n");
    out.push_str("    movk w10, #0x5A11, lsl #16\n");
    out.push_str("    cmp w9, w10\n");
    out.push_str("    b.ne 1f\n");
    out.push_str("    ldr x9, [x16]\n");
    out.push_str("    ldr x10, [x16, #8]\n");
    out.push_str("    str x10, [sp, #-16]!\n");
    out.push_str("    mov x16, x9\n");
    emit_call_target(out, "x16", args_count + 1);
    out.push_str("    b 2f\n");
    out.push_str("1:\n");
    emit_call_target(out, "x16", args_count);
    out.push_str("2:\n");
}

pub fn emit_c_function_call(
    out: &mut String,
    name: &str,
    float_args: &[bool],
    args_count: usize,
    os: OperatingSystem,
) {
    let target = if matches!(os, OperatingSystem::MacOS) {
        format!("_{}", name)
    } else {
        name.to_string()
    };
    if matches!(os, OperatingSystem::Windows) {
        // Windows ARM64 follows the standard AAPCS64 Stage C for
        // non-variadic calls (alya-lang/alya#164): ints ride x0-x7
        // (NGRN) and floats d0-d7 (NSRN) in SEPARATE sequences, so a
        // (ptr, int, double) call routes to x0, x1, d0. The old x-mirror
        // (d2) fed the callee x0 pointer bits as a ~1e-311 denormal.
        // The MS variadic addendum uses NO FP registers (everything
        // rides xN positionally), so floats additionally home into
        // their positional x slot when it is otherwise unclaimed —
        // serving fixed and variadic callees at once. Stack-spilled
        // (>8) args ride the split frame only.
        #[derive(Clone, Copy)]
        enum Route {
            X(usize),
            D(usize),
            Stack(usize),
        }
        let mut routes = Vec::with_capacity(args_count);
        let mut x = 0usize;
        let mut d = 0usize;
        let mut spilled = 0usize;
        for i in 0..args_count {
            let is_float = float_args.get(i).copied().unwrap_or(false);
            if is_float {
                if d < 8 {
                    routes.push(Route::D(d));
                    d += 1;
                } else {
                    routes.push(Route::Stack(spilled));
                    spilled += 1;
                }
            } else if x < 8 {
                routes.push(Route::X(x));
                x += 1;
            } else {
                routes.push(Route::Stack(spilled));
                spilled += 1;
            }
        }
        let claimed_x = {
            let mut claimed = [false; 8];
            for r in &routes {
                if let Route::X(g) = r {
                    claimed[*g] = true;
                }
            }
            claimed
        };
        let stack_bytes = spilled as i32 * 8;
        let frame = stack_bytes + (16 - stack_bytes % 16) % 16;
        if frame > 0 {
            out.push_str(&format!("    sub sp, sp, #{}\n", frame));
        }
        for i in (0..args_count).rev() {
            out.push_str(&format!(
                "    ldr x9, [sp, #{}]\n",
                frame + ((args_count - 1 - i) * 16) as i32
            ));
            match routes[i] {
                Route::X(g) => out.push_str(&format!("    mov x{}, x9\n", g)),
                Route::D(f) => out.push_str(&format!("    fmov d{}, x9\n", f)),
                Route::Stack(s) => {
                    out.push_str(&format!("    str x9, [sp, #{}]\n", s as i32 * 8));
                    continue;
                }
            }
            // Positional home for the variadic addendum (x[i]), when
            // the split routing left that slot unused. A clash means
            // another arg owns x[i]: the fixed route wins, the
            // variadic shape of this arg is unserved (rare; declare
            // such callees with matching arity instead).
            if i < 8 && !claimed_x[i] {
                out.push_str(&format!("    mov x{}, x9\n", i));
            }
        }
        let call_insn = if target.starts_with('x') { "blr" } else { "bl" };
        out.push_str(&format!("    {} {}\n", call_insn, target));
        out.push_str(&format!(
            "    add sp, sp, #{}\n",
            frame + args_count as i32 * 16
        ));
        return;
    }
    // AAPCS64 (alya-lang/alya#160): ints ride x0-x7, floats d0-d7 in
    // SEPARATE sequences; each sequence's overflow spills to 8-byte
    // stack slots. The shared internal lowering packs one x-sequence
    // and can never satisfy a double-typed callee parameter.
    #[derive(Clone, Copy)]
    enum Route {
        X(usize),
        D(usize),
        Stack(usize),
    }
    let mut routes = Vec::with_capacity(args_count);
    let mut x = 0usize;
    let mut d = 0usize;
    let mut spilled = 0usize;
    for i in 0..args_count {
        let is_float = float_args.get(i).copied().unwrap_or(false);
        if is_float {
            if d < 8 {
                routes.push(Route::D(d));
                d += 1;
            } else {
                routes.push(Route::Stack(spilled));
                spilled += 1;
            }
        } else if x < 8 {
            routes.push(Route::X(x));
            x += 1;
        } else {
            routes.push(Route::Stack(spilled));
            spilled += 1;
        }
    }
    // Frame holds spilled slots; sp stays 16-byte aligned throughout
    // (every Alya stack op preserves it, so pad the frame to 16).
    let stack_bytes = spilled as i32 * 8;
    let frame = stack_bytes + (16 - stack_bytes % 16) % 16;
    if frame > 0 {
        out.push_str(&format!("    sub sp, sp, #{}\n", frame));
    }
    // Value-stack slots are 16 bytes apart, arg0 deepest: route in
    // reverse straight into each arg's precomputed home (no overlap:
    // homes live inside the frame, sources above it).
    for i in (0..args_count).rev() {
        out.push_str(&format!(
            "    ldr x9, [sp, #{}]\n",
            frame + ((args_count - 1 - i) * 16) as i32
        ));
        match routes[i] {
            Route::X(g) => out.push_str(&format!("    mov x{}, x9\n", g)),
            Route::D(f) => out.push_str(&format!("    fmov d{}, x9\n", f)),
            Route::Stack(s) => out.push_str(&format!("    str x9, [sp, #{}]\n", s as i32 * 8)),
        }
    }
    let call_insn = if target.starts_with('x') { "blr" } else { "bl" };
    out.push_str(&format!("    {} {}\n", call_insn, target));
    out.push_str(&format!(
        "    add sp, sp, #{}\n",
        frame + args_count as i32 * 16
    ));
}

pub fn emit_stack_restore(out: &mut String, delta: i32) {
    out.push_str(&format!("    add sp, sp, #{}\n", delta));
}
