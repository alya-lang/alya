pub fn emit_load_num(out: &mut String, val: i64) {
    if val == 0 {
        out.push_str("    xor %eax, %eax\n");
    } else {
        out.push_str(&format!("    mov ${}, %eax\n", val as i32));
    }
}

pub fn emit_load_float(out: &mut String, val: f64) {
    let bits = val.to_bits();
    let low = (bits & 0xFFFFFFFF) as u32;
    let high = ((bits >> 32) & 0xFFFFFFFF) as u32;
    out.push_str("    sub $8, %esp\n");
    out.push_str(&format!("    movl ${}, (%esp)\n", low as i32));
    out.push_str(&format!("    movl ${}, 4(%esp)\n", high as i32));
    out.push_str("    movsd (%esp), %xmm0\n");
    out.push_str("    add $8, %esp\n");
}

pub fn emit_int_to_float(out: &mut String) {
    out.push_str("    cvtsi2sd %eax, %xmm0\n");
}

pub fn emit_float_to_int(out: &mut String) {
    out.push_str("    cvttsd2si %xmm0, %eax\n");
}

pub fn emit_load_str_label(out: &mut String, label: &str) {
    out.push_str(&format!("    mov ${}, %eax\n", label));
}

pub fn emit_load_var(out: &mut String, offset: i32) {
    out.push_str(&format!("    mov -{}(%ebp), %eax\n", offset));
}

pub fn emit_load_var_to_scratch(out: &mut String, offset: i32, is_float: bool) {
    if is_float {
        out.push_str(&format!("    movsd -{}(%ebp), %xmm1\n", offset));
    } else {
        out.push_str(&format!("    mov -{}(%ebp), %ebx\n", offset));
    }
}

pub fn emit_store_var(out: &mut String, offset: i32) {
    out.push_str(&format!("    mov %eax, -{}(%ebp)\n", offset));
}

pub fn emit_store_var_float(out: &mut String, offset: i32) {
    out.push_str(&format!("    movsd %xmm0, -{}(%ebp)\n", offset));
}

pub fn emit_allocate_var(out: &mut String, stack_offset: &mut i32) {
    *stack_offset += 4;
    out.push_str("    push %eax\n");
}

pub fn emit_push_temp(out: &mut String) {
    out.push_str("    push %eax\n");
}

pub fn emit_pop_temp(out: &mut String) {
    out.push_str("    pop %eax\n");
}

pub fn emit_load_global(out: &mut String, symbol: &str) {
    out.push_str(&format!("    movl {}, %eax\n", symbol));
}

pub fn emit_store_global(out: &mut String, symbol: &str) {
    out.push_str(&format!("    movl %eax, {}\n", symbol));
}
