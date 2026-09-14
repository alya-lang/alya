pub fn emit_call_printf(out: &mut String) {
    out.push_str("    call printf\n");
}

pub fn emit_say_str(out: &mut String, label: &str, fmt_label: &str) {
    out.push_str(&format!("    push ${}\n", label));
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str("    add $8, %esp\n");
}

pub fn emit_say_str_lit(out: &mut String, label: &str) {
    out.push_str(&format!("    push ${}\n", label));
    emit_call_printf(out);
    out.push_str("    add $4, %esp\n");
}

pub fn emit_say_offset(out: &mut String, offset: i32, fmt_label: &str) {
    out.push_str(&format!("    push -{}(%ebp)\n", offset));
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str("    add $8, %esp\n");
}

pub fn emit_say_num_const(out: &mut String, val: i64, fmt_label: &str) {
    out.push_str(&format!("    push ${}\n", val));
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str("    add $8, %esp\n");
}

pub fn emit_say_acc(out: &mut String, fmt_label: &str) {
    out.push_str("    push %eax\n");
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str("    add $8, %esp\n");
}

pub fn emit_say_float(out: &mut String, fmt_label: &str) {
    out.push_str("    sub $8, %esp\n");
    out.push_str("    movsd %xmm0, (%esp)\n");
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str("    add $12, %esp\n");
}

pub fn emit_say_interpolated_call(out: &mut String, fmt_label: &str, count: usize) {
    out.push_str(&format!("    push ${}\n", fmt_label));
    emit_call_printf(out);
    out.push_str(&format!("    add ${}, %esp\n", (count + 1) * 4));
}

pub fn emit_string_concat_call(out: &mut String) {
    out.push_str("    mov %eax, %edx\n");
    out.push_str("    pop %eax\n");
    out.push_str("    push %edx\n");
    out.push_str("    push %eax\n");
    out.push_str("    call alya_concat\n");
    out.push_str("    add $8, %esp\n");
}
