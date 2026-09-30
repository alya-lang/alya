use crate::ast::BinaryOp;
use crate::codegen::kinds::{KIND_FLOAT, KIND_UNKNOWN};

pub fn emit_try_begin(out: &mut String, catch_label: &str) {
    // Per-thread catch frames (alya-lang/alya#65): cdecl passes
    // (handler, sp, bp) at ebp+8/12/16, so push right-to-left with
    // the handler last (top). The entry sp is captured before any
    // push; the runtime records all three in the current thread's
    // block.
    out.push_str("    mov %esp, %eax\n");
    out.push_str("    push %ebp\n");
    out.push_str("    push %eax\n");
    out.push_str(&format!("    mov ${}, %eax\n", catch_label));
    out.push_str("    push %eax\n");
    out.push_str("    call alya_try_begin\n");
    out.push_str("    add $12, %esp\n");
}

pub fn emit_try_end(out: &mut String, end_label: &str, stack_delta: i32) {
    out.push_str("    call alya_try_end\n");
    if stack_delta > 0 {
        out.push_str(&format!("    add ${}, %esp\n", stack_delta));
    }
    out.push_str(&format!("    jmp {}\n", end_label));
}

pub fn emit_catch_begin(out: &mut String, catch_label: &str) {
    out.push_str(&format!("{}:\n", catch_label));
}

pub fn emit_catch_load_err(out: &mut String) {
    out.push_str("    call alya_catch_msg\n");
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

pub fn emit_array_set_imm(out: &mut String, index: usize, kind: i64) {
    // Phase 1 x86 port (#39): 8-byte slots with kind sidecar. The value
    // arrives in %xmm0 for floats, %eax otherwise (see value_kind_tag).
    out.push_str("    mov (%esp), %edx\n");
    if kind == KIND_FLOAT {
        out.push_str("    mov 8(%edx), %edx\n");
        out.push_str(&format!("    movsd %xmm0, {}(%edx)\n", index * 8));
    } else {
        out.push_str("    mov %eax, %ecx\n");
        out.push_str("    sar $31, %ecx\n");
        out.push_str("    mov 8(%edx), %edx\n");
        out.push_str(&format!("    mov %eax, {}(%edx)\n", index * 8));
        out.push_str(&format!("    mov %ecx, {}(%edx)\n", index * 8 + 4));
    }
    out.push_str("    mov (%esp), %edx\n");
    out.push_str("    mov 12(%edx), %edx\n");
    out.push_str(&format!("    movb ${}, {}(%edx)\n", kind, index));
}

pub fn emit_array_get(out: &mut String) {
    out.push_str("    mov %eax, %ecx\n");
    out.push_str("    pop %edx\n");
    out.push_str("    test %ecx, %ecx\n");
    out.push_str("    jns 1f\n");
    out.push_str("    add (%edx), %ecx\n");
    out.push_str("1:\n");
    out.push_str("    cmp (%edx), %ecx\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    // Contract (Phase 1 x86 port): lo in %eax, hi in %ecx, f64 in
    // %xmm0, slot kind in %edx. Mirrors x64 (%rax/%edx).
    out.push_str("    push %ebx\n");
    out.push_str("    mov 8(%edx), %ebx\n");
    out.push_str("    mov (%ebx, %ecx, 8), %eax\n");
    out.push_str("    mov 4(%ebx, %ecx, 8), %ebx\n");
    out.push_str("    mov 12(%edx), %edx\n");
    out.push_str("    movzbl (%edx, %ecx), %edx\n");
    out.push_str("    mov %ebx, %ecx\n");
    out.push_str("    movd %eax, %xmm0\n");
    out.push_str("    movd %ecx, %xmm1\n");
    out.push_str("    punpckldq %xmm1, %xmm0\n");
    out.push_str("    pop %ebx\n");
}

pub fn emit_array_set(out: &mut String, kind: i64) {
    // Phase 1 x86 port (#39): like set_imm but the index arrives on the
    // stack. Value in %xmm0 for floats, %eax otherwise.
    if kind == KIND_FLOAT {
        out.push_str("    pop %eax\n");
        out.push_str("    pop %edx\n");
    } else {
        out.push_str("    mov %eax, %ebx\n");
        out.push_str("    pop %eax\n");
        out.push_str("    pop %edx\n");
    }
    out.push_str("    test %eax, %eax\n");
    out.push_str("    jns 1f\n");
    out.push_str("    add (%edx), %eax\n");
    out.push_str("1:\n");
    out.push_str("    cmp (%edx), %eax\n");
    out.push_str("    jae alya_error_index_out_of_bounds\n");
    if kind == KIND_FLOAT {
        out.push_str("    push %ebx\n");
        out.push_str("    mov 8(%edx), %ebx\n");
        out.push_str("    movsd %xmm0, (%ebx, %eax, 8)\n");
        out.push_str("    mov 12(%edx), %ebx\n");
        out.push_str(&format!("    movb ${}, (%ebx, %eax)\n", kind));
        out.push_str("    pop %ebx\n");
    } else {
        out.push_str("    push %esi\n");
        out.push_str("    push %ecx\n");
        out.push_str("    mov %ebx, %ecx\n");
        out.push_str("    sar $31, %ecx\n");
        out.push_str("    mov 8(%edx), %esi\n");
        out.push_str("    mov %ebx, (%esi, %eax, 8)\n");
        out.push_str("    mov %ecx, 4(%esi, %eax, 8)\n");
        out.push_str("    mov 12(%edx), %esi\n");
        out.push_str(&format!("    movb ${}, (%esi, %eax)\n", kind));
        out.push_str("    pop %ecx\n");
        out.push_str("    pop %esi\n");
    }
}

/// Pushes a map/array value as (kind, hi, lo) for the 5-arg x86
/// `fn_set` / 4-arg `alya_array_push` calls (Phase 1 x86 port, #39).
/// The value was just generated: floats live in %xmm0, everything else
/// in %eax. `kind` is the static `value_kind_tag`, so kind == FLOAT
/// exactly when %xmm0 holds the value.
pub fn emit_value_lo_hi_kind(out: &mut String, kind: i64) {
    if kind == KIND_FLOAT {
        out.push_str("    sub $8, %esp\n");
        out.push_str("    movsd %xmm0, (%esp)\n");
        out.push_str("    mov (%esp), %eax\n");
        out.push_str("    mov 4(%esp), %ecx\n");
        out.push_str("    add $8, %esp\n");
    } else if kind == KIND_UNKNOWN {
        // Truly dynamic values: keep today's raw-%eax store, zero-high.
        out.push_str("    xor %ecx, %ecx\n");
    } else {
        out.push_str("    mov %eax, %ecx\n");
        out.push_str("    sar $31, %ecx\n");
    }
    out.push_str(&format!("    push ${}\n", kind));
    out.push_str("    push %ecx\n");
    out.push_str("    push %eax\n");
}

pub fn emit_array_push(out: &mut String, kind: i64) {
    // Phase 1 x86 port (#39): alya_array_push(arr, lo, hi, kind).
    // The array handle is on the stack; the value was just generated.
    out.push_str("    pop %edx\n");
    emit_value_lo_hi_kind(out, kind);
    out.push_str("    push %edx\n");
    out.push_str("    call alya_array_push\n");
    out.push_str("    add $16, %esp\n");
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

/// Field count; the runtime sizes uniform 8-byte slots itself
/// (alya-lang/alya#62).
pub fn emit_struct_new(out: &mut String, desc_label: &str, field_count: usize) {
    out.push_str(&format!("    push ${}\n", field_count));
    out.push_str(&format!("    push ${}\n", desc_label));
    out.push_str("    call alya_struct_new\n");
    out.push_str("    add $8, %esp\n");
}

pub fn emit_fat_ptr_new(out: &mut String, vtable_label: &str) {
    out.push_str(&format!("    push ${}\n", vtable_label));
    out.push_str("    push %eax\n");
    out.push_str("    call alya_fat_ptr_new\n");
    out.push_str("    add $8, %esp\n");
}

/// x86 struct fields live in uniform 8-byte slots (like x64,
/// alya-lang/alya#62). Loads set the double in `%xmm0` and mirror the low
/// word into `%eax`; stores take the double from `%xmm0` (call sites
/// materialize it for non-float values).
pub fn emit_struct_field_get(out: &mut String, field_idx: usize) {
    out.push_str(&format!("    movsd {}(%eax), %xmm0\n", (field_idx + 1) * 8));
    out.push_str(&format!("    movl {}(%eax), %eax\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set_imm(out: &mut String, field_idx: usize) {
    out.push_str("    movl (%esp), %edx\n");
    out.push_str(&format!("    movsd %xmm0, {}(%edx)\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set(out: &mut String, field_idx: usize) {
    out.push_str("    pop %edx\n");
    out.push_str(&format!("    movsd %xmm0, {}(%edx)\n", (field_idx + 1) * 8));
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
    is_float_var: bool,
    map_val_is_float: bool,
    map_val_is_string: bool,
) {
    // Phase 1 x86 port (#39): 8-byte slots with sidecar/entry kinds.
    // Array elements convert to the loop variable's static type;
    // unknown kinds (0) keep the raw value (status quo).
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
    out.push_str("    movl (%edx, %ecx, 8), %eax\n");
    out.push_str("    movl 4(%edx, %ecx, 8), %edx\n");
    out.push_str("    movd %eax, %xmm0\n");
    out.push_str("    movd %edx, %xmm1\n");
    out.push_str("    punpckldq %xmm1, %xmm0\n");
    out.push_str(&format!("    movl -{}(%ebp), %edx\n", arr_offset));
    out.push_str("    movl 12(%edx), %edx\n");
    out.push_str("    movzbl (%edx, %ecx), %edx\n");
    if is_float_var {
        out.push_str("    cmpl $1, %edx\n");
        out.push_str("    jne 1f\n");
        out.push_str("    cvtsi2sd %eax, %xmm0\n");
        out.push_str("1:\n");
    } else {
        out.push_str("    cmpl $2, %edx\n");
        out.push_str("    jne 1f\n");
        out.push_str("    cvttsd2si %xmm0, %eax\n");
        out.push_str("1:\n");
    }
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movl %ecx, -{}(%ebp)\n", var_offset));
        if is_float_var {
            out.push_str(&format!("    movsd %xmm0, -{}(%ebp)\n", v_off));
        } else {
            out.push_str(&format!("    movl %eax, -{}(%ebp)\n", v_off));
        }
    } else if is_float_var {
        out.push_str(&format!("    movsd %xmm0, -{}(%ebp)\n", var_offset));
    } else {
        out.push_str(&format!("    movl %eax, -{}(%ebp)\n", var_offset));
    }
    out.push_str(&format!("    jmp {}\n", done_label));

    // MAP (20-byte entries: val64 @0, key @8, state @12, tag @16)
    out.push_str(&format!("{}:\n", map_label));
    let scan_label = format!("{}_scan", map_label);
    let found_label = format!("{}_found", map_label);
    out.push_str("    movl 4(%eax), %edx\n");
    out.push_str(&format!("    movl -{}(%ebp), %ecx\n", idx_offset));
    out.push_str(&format!("{}:\n", scan_label));
    out.push_str("    cmpl %edx, %ecx\n");
    out.push_str(&format!("    jge {}\n", end_label));
    out.push_str("    imul $20, %ecx, %edi\n");
    out.push_str("    add 8(%eax), %edi\n");
    out.push_str("    cmpl $1, 12(%edi)\n");
    out.push_str(&format!("    je {}\n", found_label));
    out.push_str("    inc %ecx\n");
    out.push_str(&format!("    jmp {}\n", scan_label));
    out.push_str(&format!("{}:\n", found_label));
    out.push_str(&format!("    movl %ecx, -{}(%ebp)\n", idx_offset));
    out.push_str("    movl 8(%edi), %edx\n");
    if let Some(v_off) = val_offset {
        out.push_str(&format!("    movl %edx, -{}(%ebp)\n", var_offset));
        out.push_str("    movl (%edi), %eax\n");
        out.push_str("    movl 4(%edi), %ecx\n");
        if !map_val_is_string {
            out.push_str("    movd %eax, %xmm0\n");
            out.push_str("    movd %ecx, %xmm1\n");
            out.push_str("    punpckldq %xmm1, %xmm0\n");
            out.push_str("    movl 16(%edi), %edx\n");
            if map_val_is_float {
                out.push_str("    cmpl $1, %edx\n");
                out.push_str("    jne 1f\n");
                out.push_str("    cvtsi2sd %eax, %xmm0\n");
                out.push_str("1:\n");
            } else {
                out.push_str("    cmpl $2, %edx\n");
                out.push_str("    jne 1f\n");
                out.push_str("    cvttsd2si %xmm0, %eax\n");
                out.push_str("1:\n");
            }
        }
        if map_val_is_float {
            out.push_str(&format!("    movsd %xmm0, -{}(%ebp)\n", v_off));
        } else {
            out.push_str(&format!("    movl %eax, -{}(%ebp)\n", v_off));
        }
    } else {
        out.push_str(&format!("    movl %edx, -{}(%ebp)\n", var_offset));
    }

    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_string_equality_call(out: &mut String, op: BinaryOp) {
    // The left operand is already on the stack (generic binop pushes it),
    // the right one is in %eax: reorder to fn_strcmp(left, right), like
    // x64's (rdi, rsi). The old order passed (right, left), which is
    // invisible for ==/!= but flips </> (alya-lang/alya#59 follow-up).
    out.push_str("    mov %eax, %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    push %edx\n");
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
