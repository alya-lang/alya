use crate::ast::{BinaryOp, UnaryOp};

pub fn emit_binary_op(out: &mut String, op: BinaryOp) {
    out.push_str("    ldr x1, [sp], #16\n");
    match op {
        BinaryOp::Add => out.push_str("    add x0, x1, x0\n"),
        BinaryOp::Subtract => out.push_str("    sub x0, x1, x0\n"),
        BinaryOp::Multiply => out.push_str("    mul x0, x1, x0\n"),
        BinaryOp::Divide => {
            out.push_str("    cbz x0, alya_error_div_zero\n");
            out.push_str("    sdiv x0, x1, x0\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    cbz x0, alya_error_div_zero\n");
            out.push_str("    sdiv x2, x1, x0\n");
            out.push_str("    msub x0, x2, x0, x1\n");
        }
        BinaryOp::Equal => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, ne\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, lt\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, le\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, gt\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp x1, x0\n");
            out.push_str("    cset x0, ge\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => {
            out.push_str("    and x0, x1, x0\n");
        }
        BinaryOp::Or | BinaryOp::BitOr => {
            out.push_str("    orr x0, x1, x0\n");
        }
        BinaryOp::BitXor => {
            out.push_str("    eor x0, x1, x0\n");
        }
        BinaryOp::Shl => {
            out.push_str("    lsl x0, x1, x0\n");
        }
        BinaryOp::Shr => {
            out.push_str("    lsr x0, x1, x0\n");
        }
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_binary_op_reg(out: &mut String, op: BinaryOp) {
    match op {
        BinaryOp::Add => out.push_str("    add x0, x0, x1\n"),
        BinaryOp::Subtract => out.push_str("    sub x0, x0, x1\n"),
        BinaryOp::Multiply => out.push_str("    mul x0, x0, x1\n"),
        BinaryOp::Divide => {
            out.push_str("    cbz x1, alya_error_div_zero\n");
            out.push_str("    sdiv x0, x0, x1\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    cbz x1, alya_error_div_zero\n");
            out.push_str("    sdiv x2, x0, x1\n");
            out.push_str("    msub x0, x2, x1, x0\n");
        }
        BinaryOp::Equal => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, ne\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, lt\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, le\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, gt\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp x0, x1\n");
            out.push_str("    cset x0, ge\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => {
            out.push_str("    and x0, x0, x1\n");
        }
        BinaryOp::Or | BinaryOp::BitOr => {
            out.push_str("    orr x0, x0, x1\n");
        }
        BinaryOp::BitXor => {
            out.push_str("    eor x0, x0, x1\n");
        }
        BinaryOp::Shl => {
            out.push_str("    lsl x0, x0, x1\n");
        }
        BinaryOp::Shr => {
            out.push_str("    lsr x0, x0, x1\n");
        }
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_binary_op_imm(out: &mut String, op: BinaryOp, imm: i64) {
    match op {
        BinaryOp::Add => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    add x0, x0, #{}\n", imm));
            } else if imm < 0 && (0..=4095).contains(&(-imm)) {
                out.push_str(&format!("    sub x0, x0, #{}\n", -imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    add x0, x0, x1\n");
            }
        }
        BinaryOp::Subtract => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    sub x0, x0, #{}\n", imm));
            } else if imm < 0 && (0..=4095).contains(&(-imm)) {
                out.push_str(&format!("    add x0, x0, #{}\n", -imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    sub x0, x0, x1\n");
            }
        }
        BinaryOp::Multiply => {
            super::loads::emit_load_reg_imm64(out, "x1", imm);
            out.push_str("    mul x0, x0, x1\n");
        }
        BinaryOp::Divide => {
            if imm == 0 {
                out.push_str("    b alya_error_div_zero\n");
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    sdiv x0, x0, x1\n");
            }
        }
        BinaryOp::Modulo => {
            if imm == 0 {
                out.push_str("    b alya_error_div_zero\n");
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    sdiv x2, x0, x1\n");
                out.push_str("    msub x0, x2, x1, x0\n");
            }
        }
        BinaryOp::Equal => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, ne\n");
        }
        BinaryOp::Less => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, lt\n");
        }
        BinaryOp::LessEqual => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, le\n");
        }
        BinaryOp::Greater => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, gt\n");
        }
        BinaryOp::GreaterEqual => {
            if (0..=4095).contains(&imm) {
                out.push_str(&format!("    cmp x0, #{}\n", imm));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    cmp x0, x1\n");
            }
            out.push_str("    cset x0, ge\n");
        }
        BinaryOp::And | BinaryOp::BitAnd => {
            if imm > 0 && (imm & (imm + 1)) == 0 {
                let width = (64 - imm.leading_zeros()) as usize;
                out.push_str(&format!("    ubfx x0, x0, #0, #{}\n", width));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    and x0, x0, x1\n");
            }
        }
        BinaryOp::Or | BinaryOp::BitOr => {
            super::loads::emit_load_reg_imm64(out, "x1", imm);
            out.push_str("    orr x0, x0, x1\n");
        }
        BinaryOp::BitXor => {
            super::loads::emit_load_reg_imm64(out, "x1", imm);
            out.push_str("    eor x0, x0, x1\n");
        }
        BinaryOp::Shl => out.push_str(&format!("    lsl x0, x0, #{}\n", imm & 63)),
        BinaryOp::Shr => out.push_str(&format!("    lsr x0, x0, #{}\n", imm & 63)),
        BinaryOp::In | BinaryOp::NotIn => {}
    }
}

pub fn emit_unary_op(out: &mut String, op: UnaryOp) {
    match op {
        UnaryOp::Negate => out.push_str("    neg x0, x0\n"),
        UnaryOp::Not => {
            out.push_str("    cmp x0, #0\n");
            out.push_str("    cset x0, eq\n");
        }
        UnaryOp::BitNot => out.push_str("    mvn x0, x0\n"),
    }
}

pub fn emit_float_binary_op_reg(out: &mut String, op: BinaryOp) {
    match op {
        BinaryOp::Add => {
            out.push_str("    fadd d0, d0, d1\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Subtract => {
            out.push_str("    fsub d0, d0, d1\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Multiply => {
            out.push_str("    fmul d0, d0, d1\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Divide => {
            out.push_str("    fdiv d0, d0, d1\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    fdiv d2, d0, d1\n");
            out.push_str("    fcvtzs x2, d2\n");
            out.push_str("    scvtf d2, x2\n");
            out.push_str("    fmul d2, d2, d1\n");
            out.push_str("    fsub d0, d0, d2\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Equal => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, ne\n");
        }
        BinaryOp::Less => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, mi\n");
        }
        BinaryOp::Greater => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, gt\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, le\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    fcmp d0, d1\n");
            out.push_str("    cset x0, ge\n");
        }
        BinaryOp::And => {
            out.push_str("    and x0, x0, x1\n");
        }
        BinaryOp::Or => {
            out.push_str("    orr x0, x0, x1\n");
        }
        _ => {}
    }
}

pub fn emit_float_binary_op_imm(out: &mut String, op: BinaryOp, val: f64) {
    super::loads::emit_load_float_reg(out, "d1", val);
    emit_float_binary_op_reg(out, op);
}

pub fn emit_float_binary_op(out: &mut String, op: BinaryOp) {
    out.push_str("    ldr d1, [sp], #16\n");
    match op {
        BinaryOp::Add => {
            out.push_str("    fadd d0, d1, d0\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Subtract => {
            out.push_str("    fsub d0, d1, d0\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Multiply => {
            out.push_str("    fmul d0, d1, d0\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Divide => {
            out.push_str("    fdiv d0, d1, d0\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Modulo => {
            out.push_str("    fdiv d2, d1, d0\n");
            out.push_str("    fcvtzs x2, d2\n");
            out.push_str("    scvtf d2, x2\n");
            out.push_str("    fmul d2, d2, d0\n");
            out.push_str("    fsub d0, d1, d2\n");
            out.push_str("    fmov x0, d0\n");
        }
        BinaryOp::Equal => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, ne\n");
        }
        BinaryOp::Less => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, mi\n");
        }
        BinaryOp::Greater => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, gt\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, le\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    fcmp d1, d0\n");
            out.push_str("    cset x0, ge\n");
        }
        BinaryOp::And => {
            out.push_str("    ldr x1, [sp, #-16]\n");
            out.push_str("    and x0, x1, x0\n");
        }
        BinaryOp::Or => {
            out.push_str("    ldr x1, [sp, #-16]\n");
            out.push_str("    orr x0, x1, x0\n");
        }
        _ => {}
    }
}

pub fn emit_float_unary_op(out: &mut String, op: UnaryOp) {
    match op {
        UnaryOp::Negate => {
            out.push_str("    fneg d0, d0\n");
            out.push_str("    fmov x0, d0\n");
        }
        UnaryOp::Not => {
            out.push_str("    cmp x0, #0\n");
            out.push_str("    cset x0, eq\n");
        }
        _ => {}
    }
}

pub fn emit_bit_op_reg(out: &mut String, op: &str) {
    match op {
        "bit_and" => out.push_str("    and x0, x0, x1\n"),
        "bit_or" => out.push_str("    orr x0, x0, x1\n"),
        "bit_xor" => out.push_str("    eor x0, x0, x1\n"),
        "bit_shl" => out.push_str("    lsl x0, x0, x1\n"),
        "bit_shr" => out.push_str("    lsr x0, x0, x1\n"),
        _ => {}
    }
}

pub fn emit_bit_op_imm(out: &mut String, op: &str, imm: i64) {
    match op {
        "bit_shl" => out.push_str(&format!("    lsl x0, x0, #{}\n", imm & 63)),
        "bit_shr" => out.push_str(&format!("    lsr x0, x0, #{}\n", imm & 63)),
        "bit_and" => {
            if imm > 0 && (imm & (imm + 1)) == 0 {
                let width = (64 - imm.leading_zeros()) as usize;
                out.push_str(&format!("    ubfx x0, x0, #0, #{}\n", width));
            } else {
                super::loads::emit_load_reg_imm64(out, "x1", imm);
                out.push_str("    and x0, x0, x1\n");
            }
        }
        "bit_or" => {
            super::loads::emit_load_reg_imm64(out, "x1", imm);
            out.push_str("    orr x0, x0, x1\n");
        }
        "bit_xor" => {
            super::loads::emit_load_reg_imm64(out, "x1", imm);
            out.push_str("    eor x0, x0, x1\n");
        }
        _ => {}
    }
}

pub fn emit_bit_op(out: &mut String, op: &str) {
    out.push_str("    ldr x1, [sp], #16\n");
    match op {
        "bit_and" => out.push_str("    and x0, x1, x0\n"),
        "bit_or" => out.push_str("    orr x0, x1, x0\n"),
        "bit_xor" => out.push_str("    eor x0, x1, x0\n"),
        "bit_shl" => out.push_str("    lsl x0, x1, x0\n"),
        "bit_shr" => out.push_str("    lsr x0, x1, x0\n"),
        _ => {}
    }
}

pub fn emit_bit_not(out: &mut String) {
    out.push_str("    mvn x0, x0\n");
}
