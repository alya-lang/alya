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

pub fn emit_compare_and_jump_if_greater(out: &mut String, label: &str) {
    out.push_str("    mov %rax, %rbx\n");
    out.push_str("    pop %rax\n");
    out.push_str("    cmp %rbx, %rax\n");
    out.push_str(&format!("    jg {}\n", label));
}

pub fn emit_increment_var(out: &mut String, var_offset: i32, start_label: &str) {
    out.push_str(&format!("    addq $1, -{}(%rbp)\n", var_offset));
    out.push_str(&format!("    jmp {}\n", start_label));
}

pub fn emit_function_prologue(out: &mut String, name: &str) {
    out.push_str(&format!("\n.global fn_{}\n", name));
    out.push_str(&format!("fn_{}:\n", name));
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
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

fn emit_call_target(
    out: &mut String,
    target: &str,
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        if args_count <= 4 {
            for i in (0..args_count).rev() {
                let reg = match i {
                    0 => "%rcx",
                    1 => "%rdx",
                    2 => "%r8",
                    3 => "%r9",
                    _ => unreachable!(),
                };
                out.push_str(&format!("    pop {}\n", reg));
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
            out.push_str(&format!("    mov {}(%rsp), %rdx\n", (args_count - 2) * 8));
            out.push_str(&format!("    mov {}(%rsp), %r8\n", (args_count - 3) * 8));
            out.push_str(&format!("    mov {}(%rsp), %r9\n", (args_count - 4) * 8));
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
            let reg = match i {
                0 => "%rdi",
                1 => "%rsi",
                2 => "%rdx",
                3 => "%rcx",
                4 => "%r8",
                5 => "%r9",
                _ => unreachable!(),
            };
            out.push_str(&format!("    pop {}\n", reg));
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
        out.push_str(&format!("    mov {}(%rsp), %rsi\n", (args_count - 2) * 8));
        out.push_str(&format!("    mov {}(%rsp), %rdx\n", (args_count - 3) * 8));
        out.push_str(&format!("    mov {}(%rsp), %rcx\n", (args_count - 4) * 8));
        out.push_str(&format!("    mov {}(%rsp), %r8\n", (args_count - 5) * 8));
        out.push_str(&format!("    mov {}(%rsp), %r9\n", (args_count - 6) * 8));
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
    args_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    let target = if matches!(os, OperatingSystem::MacOS) {
        format!("_{}", name)
    } else {
        name.to_string()
    };
    emit_call_target(out, &target, args_count, stack_offset, os);
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

pub fn emit_stack_restore(out: &mut String, delta: i32) {
    out.push_str(&format!("    add ${}, %rsp\n", delta));
}
