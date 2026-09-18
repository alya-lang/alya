use crate::codegen::target::OperatingSystem;

pub fn emit_call_printf(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) {
        "_"
    } else {
        ""
    };
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call printf\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str(&format!("    call {}printf\n", p));
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_say_str(
    out: &mut String,
    label: &str,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
        out.push_str(&format!("    lea {}(%rip), %rdx\n", label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str(&format!("    lea {}(%rip), %rsi\n", label));
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_str_lit(out: &mut String, label: &str, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    lea {}(%rip), %rcx\n", label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str(&format!("    lea {}(%rip), %rdi\n", label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_offset(
    out: &mut String,
    offset: i32,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    mov -{}(%rbp), %rdx\n", offset));
        out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str(&format!("    mov -{}(%rbp), %rsi\n", offset));
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_num_const(
    out: &mut String,
    val: i64,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
        out.push_str(&format!("    mov ${}, %rdx\n", val));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str(&format!("    mov ${}, %rsi\n", val));
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_acc(out: &mut String, fmt_label: &str, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rdx\n");
        out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str("    mov %rax, %rsi\n");
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_float(out: &mut String, fmt_label: &str, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    movq %rax, %xmm1\n");
        out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
        out.push_str("    xor %rax, %rax\n");
        emit_call_printf(out, stack_offset, os);
    } else {
        out.push_str("    movq %rax, %xmm0\n");
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str("    mov $1, %al\n");
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_say_interpolated_pop_and_call(
    out: &mut String,
    fmt_label: &str,
    is_floats: &[bool],
    stack_offset: i32,
    os: OperatingSystem,
) {
    let count = is_floats.len();
    if matches!(os, OperatingSystem::Windows) {
        if count <= 3 {
            for i in (0..count).rev() {
                let reg = match i {
                    0 => "%rdx",
                    1 => "%r8",
                    2 => "%r9",
                    _ => unreachable!(),
                };
                out.push_str(&format!("    pop {}\n", reg));
                match i {
                    0 => out.push_str("    movq %rdx, %xmm1\n"),
                    1 => out.push_str("    movq %r8, %xmm2\n"),
                    2 => out.push_str("    movq %r9, %xmm3\n"),
                    _ => {}
                }
            }
            out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
            out.push_str("    xor %rax, %rax\n");
            emit_call_printf(out, stack_offset, os);
        } else {
            let extra_args = count - 3;
            let needed = 32 + extra_args as i32 * 8;
            let total_alloc = if (stack_offset + (count as i32 * 8) + needed) % 16 == 0 {
                needed
            } else {
                needed + 8
            };

            // Read register arguments from the pushed values on stack
            out.push_str(&format!("    mov {}(%rsp), %rdx\n", (count - 1) * 8));
            out.push_str("    movq %rdx, %xmm1\n");

            out.push_str(&format!("    mov {}(%rsp), %r8\n", (count - 2) * 8));
            out.push_str("    movq %r8, %xmm2\n");

            out.push_str(&format!("    mov {}(%rsp), %r9\n", (count - 3) * 8));
            out.push_str("    movq %r9, %xmm3\n");

            out.push_str(&format!("    lea {}(%rip), %rcx\n", fmt_label));
            out.push_str(&format!("    sub ${}, %rsp\n", total_alloc));

            for k in 3..count {
                let src_off = total_alloc + ((count - 1 - k) * 8) as i32;
                let dst_off = 32 + ((k - 3) * 8) as i32;
                out.push_str(&format!("    mov {}(%rsp), %rax\n", src_off));
                out.push_str(&format!("    mov %rax, {}(%rsp)\n", dst_off));
            }

            out.push_str("    xor %rax, %rax\n");
            out.push_str("    call printf\n");
            out.push_str(&format!(
                "    add ${}, %rsp\n",
                total_alloc + (count as i32 * 8)
            ));
        }
    } else {
        let mut int_reg_indices = Vec::with_capacity(count);
        let mut sse_reg_indices = Vec::with_capacity(count);
        let mut int_count = 0;
        let mut sse_count = 0;
        for &is_flt in is_floats {
            if is_flt {
                int_reg_indices.push(None);
                sse_reg_indices.push(Some(sse_count));
                sse_count += 1;
            } else {
                int_reg_indices.push(Some(int_count));
                sse_reg_indices.push(None);
                int_count += 1;
            }
        }

        for i in (0..count).rev() {
            out.push_str("    pop %rax\n");
            if let Some(s_idx) = sse_reg_indices[i] {
                if s_idx < 8 {
                    out.push_str(&format!("    movq %rax, %xmm{}\n", s_idx));
                }
            } else if let Some(i_idx) = int_reg_indices[i] {
                let reg = match i_idx {
                    0 => "%rsi",
                    1 => "%rdx",
                    2 => "%rcx",
                    3 => "%r8",
                    4 => "%r9",
                    _ => "%rsi",
                };
                out.push_str(&format!("    mov %rax, {}\n", reg));
            }
        }
        out.push_str(&format!("    lea {}(%rip), %rdi\n", fmt_label));
        out.push_str(&format!("    mov ${}, %al\n", sse_count));
        emit_call_printf(out, stack_offset, os);
    }
}

pub fn emit_string_concat_call(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    pop %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_concat\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rsi\n");
        out.push_str("    pop %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call alya_concat\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}
