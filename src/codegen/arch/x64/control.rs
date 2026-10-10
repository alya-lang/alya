use crate::codegen::target::OperatingSystem;

pub fn emit_jump_if_zero(out: &mut String, label: &str) {
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jz {}\n", label));
}

pub fn emit_jump_if_not_zero(out: &mut String, label: &str) {
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jnz {}\n", label));
}

pub fn emit_jump(out: &mut String, label: &str) {
    out.push_str(&format!("    jmp {}\n", label));
}

pub fn emit_compare_and_jump_if_greater(out: &mut String, label: &str, unsigned: bool) {
    out.push_str("    mov %rax, %rbx\n");
    out.push_str("    pop %rax\n");
    out.push_str("    cmp %rbx, %rax\n");
    if unsigned {
        out.push_str(&format!("    ja {}\n", label));
    } else {
        out.push_str(&format!("    jg {}\n", label));
    }
}

pub fn emit_compare_and_jump_if_greater_or_equal(out: &mut String, label: &str, unsigned: bool) {
    out.push_str("    mov %rax, %rbx\n");
    out.push_str("    pop %rax\n");
    out.push_str("    cmp %rbx, %rax\n");
    if unsigned {
        out.push_str(&format!("    jae {}\n", label));
    } else {
        out.push_str(&format!("    jge {}\n", label));
    }
}

pub fn emit_increment_var(out: &mut String, var_offset: i32, start_label: &str) {
    out.push_str(&format!("    addq $1, -{}(%rbp)\n", var_offset));
    out.push_str(&format!("    jmp {}\n", start_label));
}

pub fn emit_function_prologue(out: &mut String, name: &str, os: OperatingSystem) {
    out.push_str(&format!("\n.global fn_{}\n", name));
    if matches!(os, OperatingSystem::Windows) {
        // Windows x64 unwinding (alya-lang/alya#109): the OS unwinder
        // cannot walk a frame without .pdata/.xdata. Frame-pointer
        // prologue (push rbp + SET_FPREG) lets it recompute rsp from
        // rbp, ignoring body pushes/subs. No prologue stackalloc is
        // emitted (locals go through pushes), so no UWOP_ALLOC needed.
        out.push_str(&format!("    .seh_proc fn_{}\n", name));
    }
    out.push_str(&format!("fn_{}:\n", name));
    out.push_str("    push %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    .seh_pushreg %rbp\n");
    }
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    .seh_setframe %rbp, 0\n");
        out.push_str("    .seh_endprologue\n");
    }
}

/// Closes the SEH scope opened by the prologue. Call once per function
/// at its final `.text` position — never in the per-return epilogue
/// (every `return` inlines its own epilogue + `ret`).
pub fn emit_seh_endproc(out: &mut String) {
    out.push_str("    .seh_endproc\n");
}

/// Emits a global alias label for `@export("name")` (Chapter 18 §1.2).
/// Must precede the function prologue so both labels share one address.
pub fn emit_export_alias(out: &mut String, alias: &str) {
    out.push_str(&format!("\n.global {}\n", alias));
    out.push_str(&format!("{}:\n", alias));
}

pub fn emit_function_epilogue(out: &mut String) {
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n");
}

pub fn emit_function_param_push(
    out: &mut String,
    param_idx: usize,
    stack_offset: &mut i32,
    os: OperatingSystem,
) {
    *stack_offset += 8;
    if matches!(os, OperatingSystem::Windows) {
        match param_idx {
            0 => out.push_str("    push %rcx\n"),
            1 => out.push_str("    push %rdx\n"),
            2 => out.push_str("    push %r8\n"),
            3 => out.push_str("    push %r9\n"),
            _ => {
                let src_offset = 48 + (param_idx - 4) * 8;
                out.push_str(&format!("    push {}(%rbp)\n", src_offset));
            }
        }
    } else {
        match param_idx {
            0 => out.push_str("    push %rdi\n"),
            1 => out.push_str("    push %rsi\n"),
            2 => out.push_str("    push %rdx\n"),
            3 => out.push_str("    push %rcx\n"),
            4 => out.push_str("    push %r8\n"),
            5 => out.push_str("    push %r9\n"),
            _ => {
                let src_offset = 16 + (param_idx - 6) * 8;
                out.push_str(&format!("    push {}(%rbp)\n", src_offset));
            }
        }
    }
}

pub fn emit_call_target(
    out: &mut String,
    target: &str,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        if args_count <= 4 {
            for i in (0..args_count).rev() {
                let (reg, xmm) = match i {
                    0 => ("%rcx", "%xmm0"),
                    1 => ("%rdx", "%xmm1"),
                    2 => ("%r8", "%xmm2"),
                    3 => ("%r9", "%xmm3"),
                    _ => unreachable!(),
                };
                out.push_str(&format!("    pop {}\n", reg));
                out.push_str(&format!("    movq {}, {}\n", reg, xmm));
            }
            let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
            out.push_str(&format!("    sub ${}, %rsp\n", padding));
            out.push_str(&format!("    call {}\n", target));
            out.push_str(&format!("    add ${}, %rsp\n", padding));
        } else {
            let extra_args = args_count - 4;
            let needed = 32 + extra_args as i32 * 8;
            let total_alloc = if (stack_offset + args_count as i32 * 8 + needed) % 16 == 0 {
                needed
            } else {
                needed + 8
            };
            out.push_str(&format!("    mov {}(%rsp), %rcx\n", (args_count - 1) * 8));
            out.push_str("    movq %rcx, %xmm0\n");
            out.push_str(&format!("    mov {}(%rsp), %rdx\n", (args_count - 2) * 8));
            out.push_str("    movq %rdx, %xmm1\n");
            out.push_str(&format!("    mov {}(%rsp), %r8\n", (args_count - 3) * 8));
            out.push_str("    movq %r8, %xmm2\n");
            out.push_str(&format!("    mov {}(%rsp), %r9\n", (args_count - 4) * 8));
            out.push_str("    movq %r9, %xmm3\n");
            out.push_str(&format!("    sub ${}, %rsp\n", total_alloc));
            for k in 4..args_count {
                let src_off = total_alloc + ((args_count - 1 - k) * 8) as i32;
                let dst_off = 32 + ((k - 4) * 8) as i32;
                out.push_str(&format!("    mov {}(%rsp), %rax\n", src_off));
                out.push_str(&format!("    mov %rax, {}(%rsp)\n", dst_off));
            }
            out.push_str(&format!("    call {}\n", target));
            out.push_str(&format!(
                "    add ${}, %rsp\n",
                total_alloc + args_count as i32 * 8
            ));
        }
    } else if args_count <= 6 {
        for i in (0..args_count).rev() {
            let (reg, xmm) = match i {
                0 => ("%rdi", "%xmm0"),
                1 => ("%rsi", "%xmm1"),
                2 => ("%rdx", "%xmm2"),
                3 => ("%rcx", "%xmm3"),
                4 => ("%r8", "%xmm4"),
                5 => ("%r9", "%xmm5"),
                _ => unreachable!(),
            };
            out.push_str(&format!("    pop {}\n", reg));
            out.push_str(&format!("    movq {}, {}\n", reg, xmm));
        }
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str(&format!("    call {}\n", target));
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    } else {
        let extra_args = args_count - 6;
        let needed = extra_args as i32 * 8;
        let total_alloc = if (stack_offset + args_count as i32 * 8 + needed) % 16 == 0 {
            needed
        } else {
            needed + 8
        };
        out.push_str(&format!("    mov {}(%rsp), %rdi\n", (args_count - 1) * 8));
        out.push_str("    movq %rdi, %xmm0\n");
        out.push_str(&format!("    mov {}(%rsp), %rsi\n", (args_count - 2) * 8));
        out.push_str("    movq %rsi, %xmm1\n");
        out.push_str(&format!("    mov {}(%rsp), %rdx\n", (args_count - 3) * 8));
        out.push_str("    movq %rdx, %xmm2\n");
        out.push_str(&format!("    mov {}(%rsp), %rcx\n", (args_count - 4) * 8));
        out.push_str("    movq %rcx, %xmm3\n");
        out.push_str(&format!("    mov {}(%rsp), %r8\n", (args_count - 5) * 8));
        out.push_str("    movq %r8, %xmm4\n");
        out.push_str(&format!("    mov {}(%rsp), %r9\n", (args_count - 6) * 8));
        out.push_str("    movq %r9, %xmm5\n");
        out.push_str(&format!("    sub ${}, %rsp\n", total_alloc));
        for k in 6..args_count {
            let src_off = total_alloc + ((args_count - 1 - k) * 8) as i32;
            let dst_off = ((k - 6) * 8) as i32;
            out.push_str(&format!("    mov {}(%rsp), %rax\n", src_off));
            out.push_str(&format!("    mov %rax, {}(%rsp)\n", dst_off));
        }
        out.push_str(&format!("    call {}\n", target));
        out.push_str(&format!(
            "    add ${}, %rsp\n",
            total_alloc + args_count as i32 * 8
        ));
    }
}

pub fn emit_function_call(
    out: &mut String,
    name: &str,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    emit_call_target(out, &format!("fn_{}", name), args_count, stack_offset, os);
}

pub fn emit_c_function_call(
    out: &mut String,
    name: &str,
    float_args: &[bool],
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    let target = if matches!(os, OperatingSystem::MacOS) {
        format!("_{}", name)
    } else {
        name.to_string()
    };
    if matches!(os, OperatingSystem::Windows) {
        // Win64 passes one sequence (rcx,rdx,r8,r9) mirrored into
        // xmm0-3: the shared internal lowering is already correct.
        emit_call_target(out, &target, args_count, stack_offset, os);
    } else {
        emit_c_call_sysv(out, &target, float_args, args_count, stack_offset);
    }
}

/// Extern C call lowering for System V AMD64 (Linux/macOS x64,
/// alya-lang/alya#160). Unlike the internal custom ABI (one GP
/// sequence mirrored into xmm), SysV counts INTEGER args
/// (rdi,rsi,rdx,rcx,r8,r9) and SSE args (xmm0-xmm7) in SEPARATE
/// sequences and spills each sequence's overflow to the stack.
/// AL carries the number of XMM regs used: variadic callees read it,
/// fixed callees ignore it, so it is always emitted.
fn emit_c_call_sysv(
    out: &mut String,
    target: &str,
    float_args: &[bool],
    args_count: usize,
    stack_offset: i32,
) {
    const GP: [&str; 6] = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
    const FP: [&str; 8] = [
        "%xmm0", "%xmm1", "%xmm2", "%xmm3", "%xmm4", "%xmm5", "%xmm6", "%xmm7",
    ];
    #[derive(Clone, Copy)]
    enum Route {
        Gp(usize),
        Fp(usize),
        Stack(usize),
    }
    let mut routes = Vec::with_capacity(args_count);
    let mut gp = 0usize;
    let mut fp = 0usize;
    let mut spilled = 0usize;
    for i in 0..args_count {
        let is_float = float_args.get(i).copied().unwrap_or(false);
        if is_float {
            if fp < FP.len() {
                routes.push(Route::Fp(fp));
                fp += 1;
            } else {
                routes.push(Route::Stack(spilled));
                spilled += 1;
            }
        } else if gp < GP.len() {
            routes.push(Route::Gp(gp));
            gp += 1;
        } else {
            routes.push(Route::Stack(spilled));
            spilled += 1;
        }
    }
    // Frame holds the spilled stack args; padded so rsp%16==0 at the
    // call (prologue pushed rbp, so rsp%16==0 iff the effective depth
    // is 0 mod 16 — the N args are already on the stack at emission,
    // but stack_offset only counts the pre-arg depth).
    let stack_bytes = spilled as i32 * 8;
    let eff_mod = (stack_offset + args_count as i32 * 8) % 16;
    let need = (16 - eff_mod) % 16;
    let frame = stack_bytes + (need - stack_bytes % 16 + 16) % 16;
    if frame > 0 {
        out.push_str(&format!("    sub ${}, %rsp\n", frame));
    }
    // Args sit 8 bytes apart above the frame, arg0 deepest: pop in
    // reverse straight into each arg's home (no overlap: homes live
    // inside the frame, sources above it).
    for i in (0..args_count).rev() {
        out.push_str(&format!(
            "    mov {}(%rsp), %rax\n",
            frame + ((args_count - 1 - i) * 8) as i32
        ));
        match routes[i] {
            Route::Gp(g) => out.push_str(&format!("    mov %rax, {}\n", GP[g])),
            Route::Fp(f) => out.push_str(&format!("    movq %rax, {}\n", FP[f])),
            Route::Stack(s) => out.push_str(&format!("    mov %rax, {}(%rsp)\n", s as i32 * 8)),
        }
    }
    out.push_str(&format!("    mov ${}, %eax\n", fp));
    out.push_str(&format!("    call {}\n", target));
    out.push_str(&format!(
        "    add ${}, %rsp\n",
        frame + args_count as i32 * 8
    ));
}

pub fn emit_indirect_function_call(
    out: &mut String,
    var_offset: i32,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    out.push_str(&format!("    mov -{}(%rbp), %r11\n", var_offset));
    emit_call_target(out, "*%r11", args_count, stack_offset, os);
}

/// Guarded indirect call (alya-lang/alya#154): trap provably-non-function
/// callees instead of jumping into them. Small values (ints, null, bool)
/// and managed strings (str_buf, rodata) can never be code addresses, so
/// they trap; everything else keeps the legacy indirect call. Decided
/// purely on the runtime value: a variable statically typed as string may
/// still hold a reassigned function, so static types are not consulted.
/// Fresh `ro_label`/`call_label` come from the caller (one site each).
pub fn emit_guarded_indirect_function_call(
    out: &mut String,
    var_offset: i32,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
    ro_label: &str,
    call_label: &str,
) {
    out.push_str(&format!("    mov -{}(%rbp), %r11\n", var_offset));
    out.push_str("    cmp $65536, %r11\n");
    out.push_str("    jb alya_error_not_callable\n");
    out.push_str("    lea alya_str_buf(%rip), %rcx\n");
    out.push_str("    cmp %rcx, %r11\n");
    out.push_str(&format!("    jb {}\n", ro_label));
    out.push_str("    lea 67108864(%rcx), %rcx\n");
    out.push_str("    cmp %rcx, %r11\n");
    out.push_str("    jb alya_error_not_callable\n");
    out.push_str(&format!("{}:\n", ro_label));
    out.push_str("    lea alya_rodata_start(%rip), %rcx\n");
    out.push_str("    cmp %rcx, %r11\n");
    out.push_str(&format!("    jb {}\n", call_label));
    out.push_str("    lea alya_rodata_end(%rip), %rcx\n");
    out.push_str("    cmp %rcx, %r11\n");
    out.push_str("    jb alya_error_not_callable\n");
    out.push_str(&format!("{}:\n", call_label));
    emit_call_target(out, "*%r11", args_count, stack_offset, os);
}

pub fn emit_stack_restore(out: &mut String, delta: i32) {
    out.push_str(&format!("    add ${}, %rsp\n", delta));
}
