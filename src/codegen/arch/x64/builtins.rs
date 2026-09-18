use crate::ast::BinaryOp;
use crate::codegen::target::OperatingSystem;

pub fn emit_try_begin(out: &mut String, catch_label: &str) {
    out.push_str("    mov alya_catch_idx(%rip), %r8\n");
    out.push_str(&format!("    lea {}(%rip), %rax\n", catch_label));
    out.push_str("    lea alya_catch_stack_handler(%rip), %r9\n");
    out.push_str("    mov %rax, (%r9, %r8, 8)\n");
    out.push_str("    lea alya_catch_stack_sp(%rip), %r9\n");
    out.push_str("    mov %rsp, (%r9, %r8, 8)\n");
    out.push_str("    lea alya_catch_stack_bp(%rip), %r9\n");
    out.push_str("    mov %rbp, (%r9, %r8, 8)\n");
    out.push_str("    incq alya_catch_idx(%rip)\n");
}

pub fn emit_try_end(out: &mut String, end_label: &str, stack_delta: i32) {
    out.push_str("    decq alya_catch_idx(%rip)\n");
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %rsp\n", stack_delta));
    }
    out.push_str(&format!("    jmp {}\n", end_label));
}

pub fn emit_catch_begin(out: &mut String, catch_label: &str) {
    out.push_str(&format!("{}:\n", catch_label));
}

pub fn emit_catch_load_err(out: &mut String) {
    out.push_str("    mov alya_err_msg(%rip), %rax\n");
}

pub fn emit_catch_end(out: &mut String, stack_delta: i32) {
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %rsp\n", stack_delta));
    }
}

pub fn emit_array_new(out: &mut String, count: usize, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    mov ${}, %rcx\n", count));
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_array_new\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str(&format!("    mov ${}, %rdi\n", count));
        out.push_str("    call alya_array_new\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_array_set_imm(out: &mut String, index: usize) {
    out.push_str("    mov (%rsp), %rdx\n");
    out.push_str("    mov 16(%rdx), %rdx\n");
    out.push_str(&format!("    movq %rax, {}(%rdx)\n", index * 8));
}

pub fn emit_array_get(out: &mut String) {
    out.push_str("    mov %rax, %rcx\n");
    out.push_str("    pop %rdx\n");
    out.push_str("    test %rcx, %rcx\n");
    out.push_str("    jns 1f\n");
    out.push_str("    add (%rdx), %rcx\n");
    out.push_str("1:\n");
    out.push_str("    cmpq (%rdx), %rcx\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    out.push_str("    mov 16(%rdx), %rdx\n");
    out.push_str("    movq (%rdx, %rcx, 8), %rax\n");
}

pub fn emit_array_set(out: &mut String) {
    out.push_str("    mov %rax, %r8\n");
    out.push_str("    pop %rax\n");
    out.push_str("    pop %rdx\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jns 1f\n");
    out.push_str("    add (%rdx), %rax\n");
    out.push_str("1:\n");
    out.push_str("    cmpq (%rdx), %rax\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    out.push_str("    mov 16(%rdx), %rdx\n");
    out.push_str("    movq %r8, (%rdx, %rax, 8)\n");
}

pub fn emit_array_push(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    pop %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_array_push\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        out.push_str("    mov %rax, %rsi\n");
        out.push_str("    pop %rdi\n");
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call alya_array_push\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_array_pop(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_array_pop\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call alya_array_pop\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_array_len(out: &mut String) {
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jz 1f\n");
    out.push_str("    movq (%rax), %rax\n");
    out.push_str("1:\n");
}

pub fn emit_print_array(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_print_array\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call alya_print_array\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_print_map(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_print_map\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call alya_print_map\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_struct_new(
    out: &mut String,
    desc_label: &str,
    field_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    lea {}(%rip), %rcx\n", desc_label));
        out.push_str(&format!("    mov ${}, %rdx\n", field_count));
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_struct_new\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str(&format!("    lea {}(%rip), %rdi\n", desc_label));
        out.push_str(&format!("    mov ${}, %rsi\n", field_count));
        out.push_str("    call alya_struct_new\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_fat_ptr_new(
    out: &mut String,
    vtable_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    lea {}(%rip), %rdx\n", vtable_label));
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_fat_ptr_new\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    mov %rax, %rdi\n");
        out.push_str(&format!("    lea {}(%rip), %rsi\n", vtable_label));
        let p = if matches!(os, OperatingSystem::MacOS) {
            "_"
        } else {
            ""
        };
        out.push_str(&format!("    call {}alya_fat_ptr_new\n", p));
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_struct_field_get(out: &mut String, field_idx: usize) {
    out.push_str(&format!("    movq {}(%rax), %rax\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set_imm(out: &mut String, field_idx: usize) {
    out.push_str("    mov (%rsp), %rdx\n");
    out.push_str(&format!("    movq %rax, {}(%rdx)\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set(out: &mut String, field_idx: usize) {
    out.push_str("    pop %rdx\n");
    out.push_str(&format!("    movq %rax, {}(%rdx)\n", (field_idx + 1) * 8));
}

pub fn emit_print_struct(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_print_struct\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    call alya_print_struct\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn emit_for_each_load_element(
    out: &mut String,
    arr_offset: i32,
    idx_offset: i32,
    var_offset: i32,
    val_offset: Option<i32>,
    end_label: &str,
    map_label: &str,
    done_label: &str,
) {
    out.push_str(&format!("    movq -{}(%rbp), %rax\n", arr_offset));
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jz {}\n", end_label));
    out.push_str("    movq -16(%rax), %r11\n");
    out.push_str("    cmpq $0x5A110002, %r11\n");
    out.push_str(&format!("    je {}\n", map_label));

    // ARRAY
    out.push_str("    movq (%rax), %rdx\n");
    out.push_str(&format!("    movq -{}(%rbp), %rcx\n", idx_offset));
    out.push_str("    cmpq %rdx, %rcx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    movq 16(%rax), %rdx\n");
    out.push_str("    movq (%rdx, %rcx, 8), %r8\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movq %rcx, -{}(%rbp)\n", var_offset));
        out.push_str(&format!("    movq %r8, -{}(%rbp)\n", v_off));
    } else {
        out.push_str(&format!("    movq %r8, -{}(%rbp)\n", var_offset));
    }
    out.push_str(&format!("    jmp {}\n", done_label));

    // MAP
    out.push_str(&format!("{}:\n", map_label));
    let scan_label = format!("{}_scan", map_label);
    let found_label = format!("{}_found", map_label);
    out.push_str("    movq 8(%rax), %rdx\n");
    out.push_str(&format!("    movq -{}(%rbp), %rcx\n", idx_offset));
    out.push_str(&format!("{}:\n", scan_label));
    out.push_str("    cmpq %rdx, %rcx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    lea (%rcx, %rcx, 2), %r8\n");
    out.push_str("    shl $3, %r8\n");
    out.push_str("    add 16(%rax), %r8\n");
    out.push_str("    cmpq $1, 16(%r8)\n");
    out.push_str(&format!("    je {}\n", found_label));
    out.push_str("    inc %rcx\n");
    out.push_str(&format!("    jmp {}\n", scan_label));
    out.push_str(&format!("{}:\n", found_label));
    out.push_str(&format!("    movq %rcx, -{}(%rbp)\n", idx_offset));
    out.push_str("    movq (%r8), %r9\n");
    out.push_str("    movq 8(%r8), %r10\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movq %r9, -{}(%rbp)\n", var_offset));
        out.push_str(&format!("    movq %r10, -{}(%rbp)\n", v_off));
    } else {
        out.push_str(&format!("    movq %r9, -{}(%rbp)\n", var_offset));
    }

    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_string_equality_call(
    out: &mut String,
    op: BinaryOp,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    pop %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_strcmp\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rsi\n");
        out.push_str("    pop %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_strcmp\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
    match op {
        BinaryOp::Equal => {
            out.push_str("    test %rax, %rax\n    sete %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    test %rax, %rax\n    setne %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp $0, %rax\n    setl %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp $0, %rax\n    setle %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp $0, %rax\n    setg %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp $0, %rax\n    setge %al\n    movzbq %al, %rax\n");
        }
        _ => {}
    }
}

pub fn emit_in_call(out: &mut String, op: BinaryOp, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        out.push_str("    pop %rdx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_in\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rdi\n");
        out.push_str("    pop %rsi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_in\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
    if op == BinaryOp::NotIn {
        out.push_str("    test %rax, %rax\n    sete %al\n    movzbq %al, %rax\n");
    }
}

pub fn emit_char_code_at(out: &mut String, done_label: &str) {
    out.push_str("    mov %rax, %rcx\n");
    out.push_str("    pop %rdx\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str(&format!("    jz {}\n", done_label));
    out.push_str("    test %rcx, %rcx\n");
    out.push_str(&format!("    jl {}\n", done_label));
    out.push_str("    movzbl (%rdx, %rcx), %eax\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_char_code_at_direct(out: &mut String, done_label: &str) {
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %rdx, %rdx\n");
    out.push_str(&format!("    jz {}\n", done_label));
    out.push_str("    test %rcx, %rcx\n");
    out.push_str(&format!("    jl {}\n", done_label));
    out.push_str("    movzbl (%rdx, %rcx), %eax\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_call_str_to_int(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_str_to_int\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_str_to_int\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_call_str_to_float(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_str_to_float\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_str_to_float\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_rc_retain(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_rc_retain\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_rc_retain\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_rc_release(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    mov %rax, %rcx\n");
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_rc_release\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str("    mov %rax, %rdi\n");
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_rc_release\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}

pub fn emit_weak_check(out: &mut String, stack_offset: i32, os: OperatingSystem, lbl: &str) {
    let clean_lbl = lbl.trim_start_matches('.');
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jz .L_weak_done_{}\n", clean_lbl));
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    push %rax\n");
        out.push_str("    mov %rax, %rcx\n");
        let padding = if stack_offset % 16 == 0 { 40 } else { 32 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_rc_count\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
        out.push_str("    pop %rdx\n");
    } else {
        out.push_str("    push %rax\n");
        out.push_str("    mov %rax, %rdi\n");
        let misaligned = stack_offset % 16 == 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_rc_count\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
        out.push_str("    pop %rdx\n");
    }
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jnz .L_weak_alive_{}\n", clean_lbl));
    out.push_str("    xor %rax, %rax\n");
    out.push_str(&format!("    jmp .L_weak_done_{}\n", clean_lbl));
    out.push_str(&format!(".L_weak_alive_{}:\n", clean_lbl));
    out.push_str("    mov %rdx, %rax\n");
    out.push_str(&format!(".L_weak_done_{}:\n", clean_lbl));
}

pub fn emit_rc_release_stack(
    out: &mut String,
    offset: i32,
    stack_offset: i32,
    os: OperatingSystem,
) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    mov -{}(%rbp), %rcx\n", offset));
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call fn_rc_release\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        out.push_str(&format!("    mov -{}(%rbp), %rdi\n", offset));
        let misaligned = stack_offset % 16 != 0;
        if misaligned {
            out.push_str("    sub $8, %rsp\n");
        }
        out.push_str("    call fn_rc_release\n");
        if misaligned {
            out.push_str("    add $8, %rsp\n");
        }
    }
}
