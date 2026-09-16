pub fn emit_jump_if_zero(out: &mut String, label: &str) {
    out.push_str("    test %eax, %eax\n");
    out.push_str(&format!("    jz {}\n", label));
}

pub fn emit_jump_if_not_zero(out: &mut String, label: &str) {
    out.push_str("    test %eax, %eax\n");
    out.push_str(&format!("    jnz {}\n", label));
}

pub fn emit_jump(out: &mut String, label: &str) {
    out.push_str(&format!("    jmp {}\n", label));
}

pub fn emit_compare_and_jump_if_greater(out: &mut String, label: &str) {
    out.push_str("    mov %eax, %ebx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    cmp %ebx, %eax\n");
    out.push_str(&format!("    jg {}\n", label));
}

pub fn emit_increment_var(out: &mut String, var_offset: i32, start_label: &str) {
    out.push_str(&format!("    addl $1, -{}(%ebp)\n", var_offset));
    out.push_str(&format!("    jmp {}\n", start_label));
}

pub fn emit_function_prologue(out: &mut String, name: &str) {
    out.push_str(&format!("\n.global fn_{}\n", name));
    out.push_str(&format!("fn_{}:\n", name));
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
}

pub fn emit_function_epilogue(out: &mut String) {
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
}

pub fn emit_function_param_push(out: &mut String, param_idx: usize, stack_offset: &mut i32) {
    *stack_offset += 4;
    let src_offset = 8 + param_idx * 4;
    out.push_str(&format!("    push {}(%ebp)\n", src_offset));
}

pub fn emit_function_call(out: &mut String, name: &str, args_count: usize) {
    out.push_str(&format!("    call fn_{}\n", name));
    if args_count > 0 {
        out.push_str(&format!("    add ${}, %esp\n", args_count * 4));
    }
}

pub fn emit_c_function_call(out: &mut String, name: &str, args_count: usize) {
    out.push_str(&format!("    call {}\n", name));
    if args_count > 0 {
        out.push_str(&format!("    add ${}, %esp\n", args_count * 4));
    }
}

pub fn emit_indirect_function_call(out: &mut String, var_offset: i32, args_count: usize) {
    out.push_str(&format!("    mov -{}(%ebp), %ecx\n", var_offset));
    out.push_str("    call *%ecx\n");
    if args_count > 0 {
        out.push_str(&format!("    add ${}, %esp\n", args_count * 4));
    }
}

pub fn emit_stack_restore(out: &mut String, delta: i32) {
    out.push_str(&format!("    add ${}, %esp\n", delta));
}
