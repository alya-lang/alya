use super::control;
use crate::ast::BinaryOp;
use crate::codegen::kinds::{KIND_FLOAT, KIND_INT};
use crate::codegen::target::OperatingSystem;

pub fn emit_try_begin(out: &mut String, catch_label: &str, stack_offset: i32, os: OperatingSystem) {
    // Per-thread catch frames (alya-lang/alya#65): push
    // (handler, sp, bp) as call temps; the runtime records them in
    // the current thread's block. rsp math accounts for exactly one
    // push when capturing the entry sp. Internal call stays bare
    // (no Darwin `_` prefix): runtime `.global` labels are bare too
    // (see test_x64_macos_internal_symbols_no_darwin_prefix).
    out.push_str(&format!("    lea {}(%rip), %rax\n", catch_label));
    out.push_str("    push %rax\n");
    out.push_str("    lea 8(%rsp), %rax\n");
    out.push_str("    push %rax\n");
    out.push_str("    push %rbp\n");
    control::emit_call_target(out, "alya_try_begin", 3, stack_offset, os);
}

pub fn emit_try_end(
    out: &mut String,
    end_label: &str,
    stack_delta: i32,
    stack_offset: i32,
    os: OperatingSystem,
) {
    control::emit_call_target(out, "alya_try_end", 0, stack_offset, os);
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %rsp\n", stack_delta));
    }
    out.push_str(&format!("    jmp {}\n", end_label));
}

pub fn emit_catch_begin(out: &mut String, catch_label: &str) {
    out.push_str(&format!("{}:\n", catch_label));
}

pub fn emit_catch_load_err(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    control::emit_call_target(out, "alya_catch_msg", 0, stack_offset, os);
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

pub fn emit_array_set_imm(out: &mut String, index: usize, kind: i64) {
    out.push_str("    mov (%rsp), %rdx\n");
    out.push_str("    mov 24(%rdx), %rcx\n");
    out.push_str(&format!("    movb ${}, {}(%rcx)\n", kind, index));
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
    out.push_str("    push %r11\n");
    out.push_str("    mov 16(%rdx), %r11\n");
    out.push_str("    mov 24(%rdx), %rdx\n");
    out.push_str("    movzbl (%rdx, %rcx), %edx\n");
    out.push_str("    movq (%r11, %rcx, 8), %rax\n");
    out.push_str("    pop %r11\n");
}

pub fn emit_array_set(out: &mut String, kind: i64) {
    out.push_str("    mov %rax, %r8\n");
    out.push_str("    pop %rax\n");
    out.push_str("    pop %rdx\n");
    out.push_str("    test %rax, %rax\n");
    out.push_str("    jns 1f\n");
    out.push_str("    add (%rdx), %rax\n");
    out.push_str("1:\n");
    out.push_str("    cmpq (%rdx), %rax\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    out.push_str("    push %r11\n");
    out.push_str("    mov 24(%rdx), %r11\n");
    out.push_str(&format!("    movb ${}, (%r11, %rax)\n", kind));
    out.push_str("    pop %r11\n");
    out.push_str("    mov 16(%rdx), %rdx\n");
    out.push_str("    movq %r8, (%rdx, %rax, 8)\n");
}

pub fn emit_array_push(out: &mut String, stack_offset: i32, os: OperatingSystem, kind: i64) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rdx\n");
        out.push_str("    pop %rcx\n");
        out.push_str(&format!("    mov ${}, %r8\n", kind));
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_array_push\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
    } else {
        let misaligned = stack_offset % 16 != 0;
        out.push_str("    mov %rax, %rsi\n");
        out.push_str("    pop %rdi\n");
        out.push_str(&format!("    mov ${}, %rdx\n", kind));
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

// Flushes stdout via fflush(NULL) after a collection `say` so piped
// output survives a later crash (alya-lang/alya#72). Call when %rsp is
// back to its pre-print value: the caller's padding/alignment applies
// unchanged. Only clobbers caller-saved registers.
fn emit_stdout_flush_win(out: &mut String, padding: i32) {
    out.push_str(&format!("    sub ${}, %rsp\n", padding));
    out.push_str("    xor %rcx, %rcx\n");
    out.push_str("    call fflush\n");
    out.push_str(&format!("    add ${}, %rsp\n", padding));
}

fn emit_stdout_flush_sysv(out: &mut String, misaligned: bool, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) {
        "_"
    } else {
        ""
    };
    out.push_str("    xor %edi, %edi\n");
    if misaligned {
        out.push_str("    sub $8, %rsp\n");
    }
    out.push_str(&format!("    call {}fflush\n", p));
    if misaligned {
        out.push_str("    add $8, %rsp\n");
    }
}

pub fn emit_print_array(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_print_array\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
        emit_stdout_flush_win(out, padding);
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
        emit_stdout_flush_sysv(out, misaligned, os);
    }
}

pub fn emit_print_map(out: &mut String, stack_offset: i32, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        let padding = if stack_offset % 16 == 0 { 32 } else { 40 };
        out.push_str("    mov %rax, %rcx\n");
        out.push_str(&format!("    sub ${}, %rsp\n", padding));
        out.push_str("    call alya_print_map\n");
        out.push_str(&format!("    add ${}, %rsp\n", padding));
        emit_stdout_flush_win(out, padding);
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
        emit_stdout_flush_sysv(out, misaligned, os);
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
        out.push_str("    call alya_fat_ptr_new\n");
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
        emit_stdout_flush_win(out, padding);
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
        emit_stdout_flush_sysv(out, misaligned, os);
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
    is_float_var: bool,
    map_val_is_float: bool,
    map_val_is_string: bool,
) {
    out.push_str(&format!("    movq -{}(%rbp), %rax\n", arr_offset));
    out.push_str("    test %rax, %rax\n");
    out.push_str(&format!("    jz {}\n", end_label));
    out.push_str("    movl -16(%rax), %r11d\n");
    out.push_str("    cmpl $0x5A110002, %r11d\n");
    out.push_str(&format!("    je {}\n", map_label));

    // ARRAY
    out.push_str("    movq (%rax), %rdx\n");
    out.push_str(&format!("    movq -{}(%rbp), %rcx\n", idx_offset));
    out.push_str("    cmpq %rdx, %rcx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    movq 16(%rax), %rdx\n");
    out.push_str("    movq 24(%rax), %r9\n");
    out.push_str("    movq (%rdx, %rcx, 8), %r8\n");
    out.push_str("    movzbq (%r9, %rcx), %r9\n");
    // Mixed elements convert to the loop variable's static type
    // (Phase 1, #39): truncation instead of raw-bit reinterpretation.
    // Unknown kinds (0) keep the raw value (status quo).
    if is_float_var {
        out.push_str("    cvtsi2sdq %r8, %xmm0\n");
        out.push_str("    movq %xmm0, %r10\n");
        out.push_str(&format!("    cmp ${}, %r9\n", KIND_INT));
        out.push_str("    cmove %r10, %r8\n");
    } else {
        out.push_str("    movq %r8, %xmm0\n");
        out.push_str("    cvttsd2siq %xmm0, %r10\n");
        out.push_str(&format!("    cmp ${}, %r9\n", KIND_FLOAT));
        out.push_str("    cmove %r10, %r8\n");
    }
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
    out.push_str("    cmpl $1, 16(%r8)\n");
    out.push_str(&format!("    je {}\n", found_label));
    out.push_str("    inc %rcx\n");
    out.push_str(&format!("    jmp {}\n", scan_label));
    out.push_str(&format!("{}:\n", found_label));
    out.push_str(&format!("    movq %rcx, -{}(%rbp)\n", idx_offset));
    out.push_str("    movq (%r8), %r9\n");
    out.push_str("    movq 8(%r8), %r10\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movq %r9, -{}(%rbp)\n", var_offset));
        if !map_val_is_string {
            // Entry kind tag in the high 32 bits of the state word
            // (Phase 1, #39): convert to the value slot's static type.
            // Unknown tags (0) keep the raw value (status quo).
            // %rax/%rcx are dead here (map ptr/index already stored).
            out.push_str("    movl 20(%r8), %eax\n");
            if map_val_is_float {
                out.push_str("    cvtsi2sdq %r10, %xmm0\n");
                out.push_str("    movq %xmm0, %rcx\n");
                out.push_str(&format!("    cmpl ${}, %eax\n", KIND_INT));
                out.push_str("    cmove %rcx, %r10\n");
            } else {
                out.push_str("    movq %r10, %xmm0\n");
                out.push_str("    cvttsd2siq %xmm0, %rcx\n");
                out.push_str(&format!("    cmpl ${}, %eax\n", KIND_FLOAT));
                out.push_str("    cmove %rcx, %r10\n");
            }
        }
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
