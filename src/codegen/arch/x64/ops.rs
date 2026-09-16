use crate::ast::{BinaryOp, UnaryOp};

pub fn emit_binary_op(out: &mut String, op: BinaryOp) {
    out.push_str("    mov %rax, %rbx\n");
    out.push_str("    pop %rax\n");
    match op {
        BinaryOp::Add => out.push_str("    add %rbx, %rax\n"),
        BinaryOp::Subtract => out.push_str("    sub %rbx, %rax\n"),
        BinaryOp::Multiply => out.push_str("    imul %rbx, %rax\n"),
        BinaryOp::Divide => {
            out.push_str("    test %rbx, %rbx\n");
            out.push_str("    jz alya_error_div_zero\n");
            out.push_str("    cqo\n");
            out.push_str("    idiv %rbx\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    test %rbx, %rbx\n");
            out.push_str("    jz alya_error_div_zero\n");
            out.push_str("    cqo\n");
            out.push_str("    idiv %rbx\n");
            out.push_str("    mov %rdx, %rax\n");
        }
        BinaryOp::Equal => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setne %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setl %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setg %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setle %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setge %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => out.push_str("    and %rbx, %rax\n"),
        BinaryOp::Or | BinaryOp::BitOr => out.push_str("    or %rbx, %rax\n"),
        BinaryOp::BitXor => out.push_str("    xor %rbx, %rax\n"),
        BinaryOp::Shl => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shl %cl, %rax\n");
        }
        BinaryOp::Shr => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shr %cl, %rax\n");
        }
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_binary_op_reg(out: &mut String, op: BinaryOp) {
    match op {
        BinaryOp::Add => out.push_str("    add %rbx, %rax\n"),
        BinaryOp::Subtract => out.push_str("    sub %rbx, %rax\n"),
        BinaryOp::Multiply => out.push_str("    imul %rbx, %rax\n"),
        BinaryOp::Divide => {
            out.push_str("    test %rbx, %rbx\n");
            out.push_str("    jz alya_error_div_zero\n");
            out.push_str("    cqo\n");
            out.push_str("    idiv %rbx\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    test %rbx, %rbx\n");
            out.push_str("    jz alya_error_div_zero\n");
            out.push_str("    cqo\n");
            out.push_str("    idiv %rbx\n");
            out.push_str("    mov %rdx, %rax\n");
        }
        BinaryOp::Equal => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setne %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setl %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setg %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setle %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp %rbx, %rax\n");
            out.push_str("    setge %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => out.push_str("    and %rbx, %rax\n"),
        BinaryOp::Or | BinaryOp::BitOr => out.push_str("    or %rbx, %rax\n"),
        BinaryOp::BitXor => out.push_str("    xor %rbx, %rax\n"),
        BinaryOp::Shl => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shl %cl, %rax\n");
        }
        BinaryOp::Shr => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shr %cl, %rax\n");
        }
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_binary_op_imm(out: &mut String, op: BinaryOp, imm: i64) {
    let fits_i32 = (i32::MIN as i64..=i32::MAX as i64).contains(&imm);
    match op {
        BinaryOp::Add => {
            if fits_i32 {
                out.push_str(&format!("    add ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    add %rbx, %rax\n", imm));
            }
        }
        BinaryOp::Subtract => {
            if fits_i32 {
                out.push_str(&format!("    sub ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    sub %rbx, %rax\n", imm));
            }
        }
        BinaryOp::Multiply => {
            if fits_i32 {
                out.push_str(&format!("    imul ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    imul %rbx, %rax\n", imm));
            }
        }
        BinaryOp::Divide => {
            if imm == 0 {
                out.push_str("    jmp alya_error_div_zero\n");
            } else {
                out.push_str(&format!("    mov ${}, %rbx\n    cqo\n    idiv %rbx\n", imm));
            }
        }
        BinaryOp::Modulo => {
            if imm == 0 {
                out.push_str("    jmp alya_error_div_zero\n");
            } else {
                out.push_str(&format!(
                    "    mov ${}, %rbx\n    cqo\n    idiv %rbx\n    mov %rdx, %rax\n",
                    imm
                ));
            }
        }
        BinaryOp::Equal => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    sete %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    setne %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    setl %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    setle %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    setg %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            if fits_i32 {
                out.push_str(&format!("    cmp ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    cmp %rbx, %rax\n", imm));
            }
            out.push_str("    setge %al\n    movzbq %al, %rax\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => {
            if fits_i32 {
                out.push_str(&format!("    and ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    and %rbx, %rax\n", imm));
            }
        }
        BinaryOp::Or | BinaryOp::BitOr => {
            if fits_i32 {
                out.push_str(&format!("    or ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    or %rbx, %rax\n", imm));
            }
        }
        BinaryOp::BitXor => {
            if fits_i32 {
                out.push_str(&format!("    xor ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    xor %rbx, %rax\n", imm));
            }
        }
        BinaryOp::Shl => out.push_str(&format!("    shl ${}, %rax\n", imm & 63)),
        BinaryOp::Shr => out.push_str(&format!("    shr ${}, %rax\n", imm & 63)),
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_unary_op(out: &mut String, op: UnaryOp) {
    match op {
        UnaryOp::Negate => out.push_str("    neg %rax\n"),
        UnaryOp::Not => {
            out.push_str("    test %rax, %rax\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        UnaryOp::BitNot => out.push_str("    not %rax\n"),
    }
}

pub fn emit_float_binary_op_reg(out: &mut String, op: BinaryOp) {
    match op {
        BinaryOp::Add => {
            out.push_str("    addsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Subtract => {
            out.push_str("    subsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Multiply => {
            out.push_str("    mulsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Divide => {
            out.push_str("    divsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    movapd %xmm0, %xmm2\n");
            out.push_str("    divsd %xmm1, %xmm2\n");
            out.push_str("    cvttsd2siq %xmm2, %rcx\n");
            out.push_str("    cvtsi2sdq %rcx, %xmm2\n");
            out.push_str("    mulsd %xmm1, %xmm2\n");
            out.push_str("    subsd %xmm2, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Equal => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setne %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setb %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    seta %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setbe %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setae %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::And => out.push_str("    and %rbx, %rax\n"),
        BinaryOp::Or => out.push_str("    or %rbx, %rax\n"),
        _ => {}
    }
}

pub fn emit_float_binary_op_imm(out: &mut String, op: BinaryOp, val: f64) {
    let bits = val.to_bits() as i64;
    out.push_str(&format!("    movabs ${}, %rbx\n", bits));
    out.push_str("    movq %rbx, %xmm1\n");
    emit_float_binary_op_reg(out, op);
}

pub fn emit_float_binary_op(out: &mut String, op: BinaryOp) {
    out.push_str("    movq %rax, %xmm1\n");
    out.push_str("    pop %rax\n");
    out.push_str("    movq %rax, %xmm0\n");
    match op {
        BinaryOp::Add => {
            out.push_str("    addsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Subtract => {
            out.push_str("    subsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Multiply => {
            out.push_str("    mulsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Divide => {
            out.push_str("    divsd %xmm1, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    movapd %xmm0, %xmm2\n");
            out.push_str("    divsd %xmm1, %xmm2\n");
            out.push_str("    cvttsd2siq %xmm2, %rcx\n");
            out.push_str("    cvtsi2sdq %rcx, %xmm2\n");
            out.push_str("    mulsd %xmm1, %xmm2\n");
            out.push_str("    subsd %xmm2, %xmm0\n");
            out.push_str("    movq %xmm0, %rax\n");
        }
        BinaryOp::Equal => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setne %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Less => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setb %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::Greater => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    seta %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setbe %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    ucomisd %xmm1, %xmm0\n");
            out.push_str("    setae %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        BinaryOp::And => out.push_str("    and %rbx, %rax\n"),
        BinaryOp::Or => out.push_str("    or %rbx, %rax\n"),
        _ => {}
    }
}

pub fn emit_float_unary_op(out: &mut String, op: UnaryOp) {
    match op {
        UnaryOp::Negate => {
            out.push_str("    mov $0x8000000000000000, %rcx\n");
            out.push_str("    xor %rcx, %rax\n");
            out.push_str("    movq %rax, %xmm0\n");
        }
        UnaryOp::Not => {
            out.push_str("    test %rax, %rax\n");
            out.push_str("    sete %al\n");
            out.push_str("    movzbq %al, %rax\n");
        }
        _ => {}
    }
}

pub fn emit_bit_op_reg(out: &mut String, op: &str) {
    match op {
        "bit_and" => out.push_str("    and %rbx, %rax\n"),
        "bit_or" => out.push_str("    or %rbx, %rax\n"),
        "bit_xor" => out.push_str("    xor %rbx, %rax\n"),
        "bit_shl" => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shl %cl, %rax\n");
        }
        "bit_shr" => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shr %cl, %rax\n");
        }
        _ => {}
    }
}

pub fn emit_bit_op_imm(out: &mut String, op: &str, imm: i64) {
    let fits_i32 = (i32::MIN as i64..=i32::MAX as i64).contains(&imm);
    match op {
        "bit_shl" => out.push_str(&format!("    shl ${}, %rax\n", imm & 63)),
        "bit_shr" => out.push_str(&format!("    shr ${}, %rax\n", imm & 63)),
        "bit_and" => {
            if fits_i32 {
                out.push_str(&format!("    and ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    and %rbx, %rax\n", imm));
            }
        }
        "bit_or" => {
            if fits_i32 {
                out.push_str(&format!("    or ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    or %rbx, %rax\n", imm));
            }
        }
        "bit_xor" => {
            if fits_i32 {
                out.push_str(&format!("    xor ${}, %rax\n", imm));
            } else {
                out.push_str(&format!("    movabs ${}, %rbx\n    xor %rbx, %rax\n", imm));
            }
        }
        _ => {}
    }
}

pub fn emit_bit_op(out: &mut String, op: &str) {
    out.push_str("    mov %rax, %rbx\n");
    out.push_str("    pop %rax\n");
    match op {
        "bit_and" => out.push_str("    and %rbx, %rax\n"),
        "bit_or" => out.push_str("    or %rbx, %rax\n"),
        "bit_xor" => out.push_str("    xor %rbx, %rax\n"),
        "bit_shl" => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shl %cl, %rax\n");
        }
        "bit_shr" => {
            out.push_str("    mov %rbx, %rcx\n");
            out.push_str("    shr %cl, %rax\n");
        }
        _ => {}
    }
}

pub fn emit_bit_not(out: &mut String) {
    out.push_str("    not %rax\n");
}
