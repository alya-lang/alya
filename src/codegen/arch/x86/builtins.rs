use crate::ast::BinaryOp;

pub fn emit_try_begin(out: &mut String, catch_label: &str) {
    out.push_str("    mov alya_catch_idx, %ecx\n");
    out.push_str(&format!("    mov ${}, %eax\n", catch_label));
    out.push_str("    mov $alya_catch_stack_handler, %edx\n");
    out.push_str("    mov %eax, (%edx, %ecx, 4)\n");
    out.push_str("    mov $alya_catch_stack_sp, %edx\n");
    out.push_str("    mov %esp, (%edx, %ecx, 4)\n");
    out.push_str("    mov $alya_catch_stack_bp, %edx\n");
    out.push_str("    mov %ebp, (%edx, %ecx, 4)\n");
    out.push_str("    incl alya_catch_idx\n");
}

pub fn emit_try_end(out: &mut String, end_label: &str, stack_delta: i32) {
    out.push_str("    decl alya_catch_idx\n");
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %esp\n", stack_delta));
    }
    out.push_str(&format!("    jmp {}\n", end_label));
}

pub fn emit_catch_begin(out: &mut String, catch_label: &str) {
    out.push_str(&format!("{}:\n", catch_label));
}

pub fn emit_catch_load_err(out: &mut String) {
    out.push_str("    mov alya_err_msg, %eax\n");
}

pub fn emit_catch_end(out: &mut String, stack_delta: i32) {
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %esp\n", stack_delta));
    }
}

pub fn emit_array_new(out: &mut String, count: usize) {
    out.push_str(&format!("    push ${}\n", count));
    out.push_str("    call alya_array_new\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_array_set_imm(out: &mut String, index: usize) {
    out.push_str("    mov (%esp), %edx\n");
    out.push_str("    mov 8(%edx), %edx\n");
    out.push_str(&format!("    mov %eax, {}(%edx)\n", index * 4));
}

pub fn emit_array_get(out: &mut String) {
    out.push_str("    mov %eax, %ecx\n");
    out.push_str("    pop %edx\n");
    out.push_str("    cmp (%edx), %ecx\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    out.push_str("    mov 8(%edx), %edx\n");
    out.push_str("    mov (%edx, %ecx, 4), %eax\n");
}

pub fn emit_array_set(out: &mut String) {
    out.push_str("    mov %eax, %ebx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    pop %edx\n");
    out.push_str("    cmp (%edx), %eax\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    out.push_str("    mov 8(%edx), %edx\n");
    out.push_str("    mov %ebx, (%edx, %eax, 4)\n");
}

pub fn emit_array_push(out: &mut String) {
    out.push_str("    pop %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $8, %esp\n");
}

pub fn emit_array_pop(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call alya_array_pop\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_array_len(out: &mut String) {
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jz 1f\n");
    out.push_str("    mov (%eax), %eax\n");
    out.push_str("1:\n");
}

pub fn emit_print_array(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call alya_print_array\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_print_map(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call alya_print_map\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_struct_new(out: &mut String, desc_label: &str, field_count: usize) {
    out.push_str(&format!("    push ${}\n", field_count));
    out.push_str(&format!("    push ${}\n", desc_label));
    out.push_str("    call alya_struct_new\n");
    out.push_str("    add $8, %esp\n");
}

pub fn emit_struct_field_get(out: &mut String, field_idx: usize) {
    out.push_str(&format!("    movl {}(%eax), %eax\n", (field_idx + 1) * 4));
}

pub fn emit_struct_field_set_imm(out: &mut String, field_idx: usize) {
    out.push_str("    movl (%esp), %edx\n");
    out.push_str(&format!("    movl %eax, {}(%edx)\n", (field_idx + 1) * 4));
}

pub fn emit_struct_field_set(out: &mut String, field_idx: usize) {
    out.push_str("    pop %edx\n");
    out.push_str(&format!("    movl %eax, {}(%edx)\n", (field_idx + 1) * 4));
}

pub fn emit_print_struct(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call alya_print_struct\n");
    out.push_str("    add $4, %esp\n");
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
    out.push_str(&format!("    movl -{}(%ebp), %eax\n", arr_offset));
    out.push_str("    test %eax, %eax\n");
    out.push_str(&format!("    jz {}\n", end_label));
    out.push_str("    cmpl $0x5A110002, -8(%eax)\n");
    out.push_str(&format!("    je {}\n", map_label));

    // ARRAY
    out.push_str("    movl (%eax), %edx\n");
    out.push_str(&format!("    movl -{}(%ebp), %ecx\n", idx_offset));
    out.push_str("    cmpl %edx, %ecx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    movl 8(%eax), %edx\n");
    out.push_str("    movl (%edx, %ecx, 4), %eax\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movl %ecx, -{}(%ebp)\n", var_offset));
        out.push_str(&format!("    movl %eax, -{}(%ebp)\n", v_off));
    } else {
        out.push_str(&format!("    movl %eax, -{}(%ebp)\n", var_offset));
    }
    out.push_str(&format!("    jmp {}\n", done_label));

    // MAP
    out.push_str(&format!("{}:\n", map_label));
    let scan_label = format!("{}_scan", map_label);
    let found_label = format!("{}_found", map_label);
    out.push_str("    movl 4(%eax), %edx\n");
    out.push_str(&format!("    movl -{}(%ebp), %ecx\n", idx_offset));
    out.push_str(&format!("{}:\n", scan_label));
    out.push_str("    cmpl %edx, %ecx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    lea (%ecx, %ecx, 2), %edi\n");
    out.push_str("    shl $2, %edi\n");
    out.push_str("    add 8(%eax), %edi\n");
    out.push_str("    cmpl $1, 8(%edi)\n");
    out.push_str(&format!("    je {}\n", found_label));
    out.push_str("    inc %ecx\n");
    out.push_str(&format!("    jmp {}\n", scan_label));
    out.push_str(&format!("{}:\n", found_label));
    out.push_str(&format!("    movl %ecx, -{}(%ebp)\n", idx_offset));
    out.push_str("    movl (%edi), %edx\n");
    out.push_str("    movl 4(%edi), %eax\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movl %edx, -{}(%ebp)\n", var_offset));
        out.push_str(&format!("    movl %eax, -{}(%ebp)\n", v_off));
    } else {
        out.push_str(&format!("    movl %edx, -{}(%ebp)\n", var_offset));
    }

    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_string_equality_call(out: &mut String, op: BinaryOp) {
    out.push_str("    push %eax\n");
    out.push_str("    call fn_strcmp\n");
    out.push_str("    add $8, %esp\n");
    match op {
        BinaryOp::Equal => {
            out.push_str("    test %eax, %eax\n    sete %al\n    movzbl %al, %eax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    test %eax, %eax\n    setne %al\n    movzbl %al, %eax\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp $0, %eax\n    setl %al\n    movzbl %al, %eax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp $0, %eax\n    setle %al\n    movzbl %al, %eax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp $0, %eax\n    setg %al\n    movzbl %al, %eax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp $0, %eax\n    setge %al\n    movzbl %al, %eax\n");
        }
        _ => {}
    }
}

pub fn emit_in_call(out: &mut String, op: BinaryOp) {
    out.push_str("    pop %edx\n"); // edx = item
    out.push_str("    push %edx\n"); // push item
    out.push_str("    push %eax\n"); // push collection
    out.push_str("    call fn_in\n");
    out.push_str("    add $8, %esp\n");
    if op == BinaryOp::NotIn {
        out.push_str("    test %eax, %eax\n    sete %al\n    movzbl %al, %eax\n");
    }
}

pub fn emit_char_code_at(out: &mut String, done_label: &str) {
    out.push_str("    mov %eax, %ecx\n");
    out.push_str("    pop %edx\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str(&format!("    jz {}\n", done_label));
    out.push_str("    test %ecx, %ecx\n");
    out.push_str(&format!("    jl {}\n", done_label));
    out.push_str("    movzbl (%edx, %ecx), %eax\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_char_code_at_direct(out: &mut String, done_label: &str) {
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    test %edx, %edx\n");
    out.push_str(&format!("    jz {}\n", done_label));
    out.push_str("    test %ecx, %ecx\n");
    out.push_str(&format!("    jl {}\n", done_label));
    out.push_str("    movzbl (%edx, %ecx), %eax\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_call_str_to_int(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call fn_str_to_int\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_call_str_to_float(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call fn_str_to_float\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_rc_retain(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_retain\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_rc_release(out: &mut String) {
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_rc_release_stack(out: &mut String, offset: i32) {
    out.push_str(&format!("    push -{}(%ebp)\n", offset));
    out.push_str("    call fn_rc_release\n");
    out.push_str("    add $4, %esp\n");
}

pub fn emit_weak_check(out: &mut String, lbl: &str) {
    let clean_lbl = lbl.trim_start_matches('.');
    out.push_str("    test %eax, %eax\n");
    out.push_str(&format!("    jz .L_weak_done_{}\n", clean_lbl));
    out.push_str("    push %eax\n");
    out.push_str("    call fn_rc_count\n");
    out.push_str("    pop %edx\n");
    out.push_str("    test %eax, %eax\n");
    out.push_str(&format!("    jnz .L_weak_alive_{}\n", clean_lbl));
    out.push_str("    xor %eax, %eax\n");
    out.push_str(&format!("    jmp .L_weak_done_{}\n", clean_lbl));
    out.push_str(&format!(".L_weak_alive_{}:\n", clean_lbl));
    out.push_str("    mov %edx, %eax\n");
    out.push_str(&format!(".L_weak_done_{}:\n", clean_lbl));
}
