use super::{arm64, x64, x86};
use crate::ast::BinaryOp;
use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_string_equality_call(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::emit_string_equality_call(out, op),
        Architecture::X64 => x64::emit_string_equality_call(out, op, stack_offset, os),
        Architecture::ARM64 => arm64::emit_string_equality_call(out, op),
    }
}

pub fn emit_in_call(
    out: &mut String,
    arch: Architecture,
    op: BinaryOp,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::emit_in_call(out, op),
        Architecture::X64 => x64::emit_in_call(out, op, stack_offset, os),
        Architecture::ARM64 => arm64::emit_in_call(out, op),
    }
}

pub fn emit_try_begin(
    out: &mut String,
    arch: Architecture,
    catch_label: &str,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_try_begin(out, catch_label, os),
        Architecture::X64 => x64::emit_try_begin(out, catch_label),
        Architecture::X86 => x86::emit_try_begin(out, catch_label),
    }
}

pub fn emit_try_end(
    out: &mut String,
    arch: Architecture,
    end_label: &str,
    stack_delta: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_try_end(out, end_label, stack_delta, os),
        Architecture::X64 => x64::emit_try_end(out, end_label, stack_delta),
        Architecture::X86 => x86::emit_try_end(out, end_label, stack_delta),
    }
}

pub fn emit_catch_begin(out: &mut String, arch: Architecture, catch_label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_begin(out, catch_label),
        Architecture::X64 => x64::emit_catch_begin(out, catch_label),
        Architecture::X86 => x86::emit_catch_begin(out, catch_label),
    }
}

pub fn emit_catch_load_err(out: &mut String, arch: Architecture, os: OperatingSystem) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_load_err(out, os),
        Architecture::X64 => x64::emit_catch_load_err(out),
        Architecture::X86 => x86::emit_catch_load_err(out),
    }
}

pub fn emit_catch_end(out: &mut String, arch: Architecture, stack_delta: i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_end(out, stack_delta),
        Architecture::X64 => x64::emit_catch_end(out, stack_delta),
        Architecture::X86 => x86::emit_catch_end(out, stack_delta),
    }
}

pub fn emit_array_new(
    out: &mut String,
    arch: Architecture,
    count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_new(out, count),
        Architecture::X64 => x64::emit_array_new(out, count, stack_offset, os),
        Architecture::X86 => x86::emit_array_new(out, count),
    }
}

pub fn emit_array_set_imm(out: &mut String, arch: Architecture, index: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_set_imm(out, index),
        Architecture::X64 => x64::emit_array_set_imm(out, index),
        Architecture::X86 => x86::emit_array_set_imm(out, index),
    }
}

pub fn emit_array_get(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_get(out),
        Architecture::X64 => x64::emit_array_get(out),
        Architecture::X86 => x86::emit_array_get(out),
    }
}

pub fn emit_array_set(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_set(out),
        Architecture::X64 => x64::emit_array_set(out),
        Architecture::X86 => x86::emit_array_set(out),
    }
}

pub fn emit_array_push(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_push(out),
        Architecture::X64 => x64::emit_array_push(out, stack_offset, os),
        Architecture::X86 => x86::emit_array_push(out),
    }
}

pub fn emit_array_pop(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_pop(out),
        Architecture::X64 => x64::emit_array_pop(out, stack_offset, os),
        Architecture::X86 => x86::emit_array_pop(out),
    }
}

pub fn emit_array_len(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_len(out),
        Architecture::X64 => x64::emit_array_len(out),
        Architecture::X86 => x86::emit_array_len(out),
    }
}

pub fn emit_print_array(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_array(out),
        Architecture::X64 => x64::emit_print_array(out, stack_offset, os),
        Architecture::X86 => x86::emit_print_array(out),
    }
}

pub fn emit_print_map(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_map(out),
        Architecture::X64 => x64::emit_print_map(out, stack_offset, os),
        Architecture::X86 => x86::emit_print_map(out),
    }
}

pub fn emit_struct_new(
    out: &mut String,
    arch: Architecture,
    desc_label: &str,
    field_count: usize,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_new(out, desc_label, field_count, os),
        Architecture::X64 => x64::emit_struct_new(out, desc_label, field_count, stack_offset, os),
        Architecture::X86 => x86::emit_struct_new(out, desc_label, field_count),
    }
}

pub fn emit_struct_field_get(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_get(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_get(out, field_idx),
        Architecture::X86 => x86::emit_struct_field_get(out, field_idx),
    }
}

pub fn emit_struct_field_set_imm(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_set_imm(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_set_imm(out, field_idx),
        Architecture::X86 => x86::emit_struct_field_set_imm(out, field_idx),
    }
}

pub fn emit_struct_field_set(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_set(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_set(out, field_idx),
        Architecture::X86 => x86::emit_struct_field_set(out, field_idx),
    }
}

pub fn emit_print_struct(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_struct(out),
        Architecture::X64 => x64::emit_print_struct(out, stack_offset, os),
        Architecture::X86 => x86::emit_print_struct(out),
    }
}

pub fn emit_for_each_load_element(
    out: &mut String,
    arch: Architecture,
    arr_offset: i32,
    idx_offset: i32,
    var_offset: i32,
    val_offset: Option<i32>,
    end_label: &str,
    map_label: &str,
    done_label: &str,
) {
    match arch {
        Architecture::X86 => x86::emit_for_each_load_element(
            out, arr_offset, idx_offset, var_offset, val_offset, end_label, map_label, done_label,
        ),
        Architecture::X64 => x64::emit_for_each_load_element(
            out, arr_offset, idx_offset, var_offset, val_offset, end_label, map_label, done_label,
        ),
        Architecture::ARM64 => arm64::emit_for_each_load_element(
            out, arr_offset, idx_offset, var_offset, val_offset, end_label, map_label, done_label,
        ),
    }
}

pub fn emit_char_code_at(out: &mut String, arch: Architecture, done_label: &str) {
    match arch {
        Architecture::X86 => x86::builtins::emit_char_code_at(out, done_label),
        Architecture::X64 => x64::builtins::emit_char_code_at(out, done_label),
        Architecture::ARM64 => arm64::builtins::emit_char_code_at(out, done_label),
    }
}

pub fn emit_char_code_at_direct(out: &mut String, arch: Architecture, done_label: &str) {
    match arch {
        Architecture::X86 => x86::builtins::emit_char_code_at_direct(out, done_label),
        Architecture::X64 => x64::builtins::emit_char_code_at_direct(out, done_label),
        Architecture::ARM64 => arm64::builtins::emit_char_code_at_direct(out, done_label),
    }
}

pub fn emit_call_str_to_int(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::builtins::emit_call_str_to_int(out),
        Architecture::X64 => x64::builtins::emit_call_str_to_int(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_call_str_to_int(out),
    }
}

pub fn emit_call_str_to_float(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::builtins::emit_call_str_to_float(out),
        Architecture::X64 => x64::builtins::emit_call_str_to_float(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_call_str_to_float(out),
    }
}

pub fn emit_rc_retain(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::builtins::emit_rc_retain(out),
        Architecture::X64 => x64::builtins::emit_rc_retain(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_retain(out),
    }
}

pub fn emit_rc_release(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::builtins::emit_rc_release(out),
        Architecture::X64 => x64::builtins::emit_rc_release(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_release(out),
    }
}

pub fn emit_rc_release_stack(
    out: &mut String,
    arch: Architecture,
    offset: i32,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X86 => x86::builtins::emit_rc_release_stack(out, offset),
        Architecture::X64 => x64::builtins::emit_rc_release_stack(out, offset, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_release_stack(out, offset),
    }
}
