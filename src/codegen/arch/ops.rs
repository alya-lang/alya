use super::{arm64, x64};
use crate::ast::{BinaryOp, UnaryOp};
use crate::codegen::target::Architecture;

pub fn emit_binary_op(out: &mut String, arch: Architecture, op: BinaryOp, unsigned: bool) {
    match arch {
        Architecture::ARM64 => arm64::emit_binary_op(out, op, unsigned),
        Architecture::X64 => x64::emit_binary_op(out, op, unsigned),
    }
}

pub fn emit_binary_op_reg(out: &mut String, arch: Architecture, op: BinaryOp, unsigned: bool) {
    match arch {
        Architecture::ARM64 => arm64::emit_binary_op_reg(out, op, unsigned),
        Architecture::X64 => x64::emit_binary_op_reg(out, op, unsigned),
    }
}

pub fn emit_binary_op_imm(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    imm: i64,
    unsigned: bool,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_binary_op_imm(out, op, imm, unsigned),
        Architecture::X64 => x64::emit_binary_op_imm(out, op, imm, unsigned),
    }
}

pub fn emit_unary_op(out: &mut String, arch: Architecture, op: UnaryOp) {
    match arch {
        Architecture::ARM64 => arm64::emit_unary_op(out, op),
        Architecture::X64 => x64::emit_unary_op(out, op),
    }
}

pub fn emit_float_binary_op(out: &mut String, arch: Architecture, op: BinaryOp) {
    match arch {
        Architecture::ARM64 => arm64::emit_float_binary_op(out, op),
        Architecture::X64 => x64::emit_float_binary_op(out, op),
    }
}

pub fn emit_float_binary_op_reg(out: &mut String, arch: Architecture, op: BinaryOp) {
    match arch {
        Architecture::ARM64 => arm64::emit_float_binary_op_reg(out, op),
        Architecture::X64 => x64::emit_float_binary_op_reg(out, op),
    }
}

pub fn emit_float_binary_op_imm(out: &mut String, arch: Architecture, op: BinaryOp, val: f64) {
    match arch {
        Architecture::ARM64 => arm64::emit_float_binary_op_imm(out, op, val),
        Architecture::X64 => x64::emit_float_binary_op_imm(out, op, val),
    }
}

pub fn emit_float_unary_op(out: &mut String, arch: Architecture, op: UnaryOp) {
    match arch {
        Architecture::ARM64 => arm64::emit_float_unary_op(out, op),
        Architecture::X64 => x64::emit_float_unary_op(out, op),
    }
}

pub fn emit_bit_op(out: &mut String, arch: Architecture, op: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_bit_op(out, op),
        Architecture::X64 => x64::emit_bit_op(out, op),
    }
}

pub fn emit_bit_op_reg(out: &mut String, arch: Architecture, op: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_bit_op_reg(out, op),
        Architecture::X64 => x64::emit_bit_op_reg(out, op),
    }
}

pub fn emit_bit_op_imm(out: &mut String, arch: Architecture, op: &str, imm: i64) {
    match arch {
        Architecture::ARM64 => arm64::emit_bit_op_imm(out, op, imm),
        Architecture::X64 => x64::emit_bit_op_imm(out, op, imm),
    }
}

pub fn emit_bit_not(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_bit_not(out),
        Architecture::X64 => x64::emit_bit_not(out),
    }
}
