use super::{arm64, x64};
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
        Architecture::X64 => x64::emit_in_call(out, op, stack_offset, os),
        Architecture::ARM64 => arm64::emit_in_call(out, op),
    }
}

pub fn emit_try_begin(
    out: &mut String,
    arch: Architecture,
    catch_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_try_begin(out, catch_label, os),
        Architecture::X64 => x64::emit_try_begin(out, catch_label, stack_offset, os),
    }
}

pub fn emit_try_end(
    out: &mut String,
    arch: Architecture,
    end_label: &str,
    stack_delta: i32,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_try_end(out, end_label, stack_delta, os),
        Architecture::X64 => x64::emit_try_end(out, end_label, stack_delta, stack_offset, os),
    }
}

pub fn emit_catch_begin(out: &mut String, arch: Architecture, catch_label: &str) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_begin(out, catch_label),
        Architecture::X64 => x64::emit_catch_begin(out, catch_label),
    }
}

pub fn emit_catch_load_err(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_load_err(out, os),
        Architecture::X64 => x64::emit_catch_load_err(out, stack_offset, os),
    }
}

pub fn emit_catch_end(out: &mut String, arch: Architecture, stack_delta: i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_catch_end(out, stack_delta),
        Architecture::X64 => x64::emit_catch_end(out, stack_delta),
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
    }
}

pub fn emit_array_set_imm(out: &mut String, arch: Architecture, index: usize, kind: i64) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_set_imm(out, index, kind),
        Architecture::X64 => x64::emit_array_set_imm(out, index, kind),
    }
}

pub fn emit_array_get(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_get(out),
        Architecture::X64 => x64::emit_array_get(out),
    }
}

pub fn emit_array_set(out: &mut String, arch: Architecture, kind: i64) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_set(out, kind),
        Architecture::X64 => x64::emit_array_set(out, kind),
    }
}

pub fn emit_array_push(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
    kind: i64,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_push(out, kind),
        Architecture::X64 => x64::emit_array_push(out, stack_offset, os, kind),
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
    }
}

pub fn emit_array_len(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_array_len(out),
        Architecture::X64 => x64::emit_array_len(out),
    }
}

pub fn emit_print_array(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_array(out, os),
        Architecture::X64 => x64::emit_print_array(out, stack_offset, os),
    }
}

pub fn emit_print_map(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_map(out, os),
        Architecture::X64 => x64::emit_print_map(out, stack_offset, os),
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
    }
}

pub fn emit_fat_ptr_new(
    out: &mut String,
    arch: Architecture,
    vtable_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_fat_ptr_new(out, vtable_label, os),
        Architecture::X64 => x64::emit_fat_ptr_new(out, vtable_label, stack_offset, os),
    }
}

pub fn emit_struct_field_get(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_get(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_get(out, field_idx),
    }
}

pub fn emit_struct_field_set_imm(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_set_imm(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_set_imm(out, field_idx),
    }
}

pub fn emit_struct_field_set(out: &mut String, arch: Architecture, field_idx: usize) {
    match arch {
        Architecture::ARM64 => arm64::emit_struct_field_set(out, field_idx),
        Architecture::X64 => x64::emit_struct_field_set(out, field_idx),
    }
}

pub fn emit_print_struct(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_print_struct(out, os),
        Architecture::X64 => x64::emit_print_struct(out, stack_offset, os),
    }
}

#[allow(clippy::too_many_arguments)]
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
    is_float_var: bool,
    map_val_is_float: bool,
    map_val_is_string: bool,
) {
    match arch {
        Architecture::X64 => x64::emit_for_each_load_element(
            out,
            arr_offset,
            idx_offset,
            var_offset,
            val_offset,
            end_label,
            map_label,
            done_label,
            is_float_var,
            map_val_is_float,
            map_val_is_string,
        ),
        Architecture::ARM64 => arm64::emit_for_each_load_element(
            out,
            arr_offset,
            idx_offset,
            var_offset,
            val_offset,
            end_label,
            map_label,
            done_label,
            is_float_var,
            map_val_is_float,
            map_val_is_string,
        ),
    }
}

pub fn emit_char_code_at(out: &mut String, arch: Architecture, done_label: &str) {
    match arch {
        Architecture::X64 => x64::builtins::emit_char_code_at(out, done_label),
        Architecture::ARM64 => arm64::builtins::emit_char_code_at(out, done_label),
    }
}

pub fn emit_char_code_at_direct(out: &mut String, arch: Architecture, done_label: &str) {
    match arch {
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
        Architecture::X64 => x64::builtins::emit_rc_retain(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_retain(out),
    }
}

/// Probe-free retain for proven-heap values: same shape as
/// `emit_rc_retain`, but calls `fn_rc_retain_direct` (no readability
/// probe, no syscall). Call ONLY when `value_proven_heap(value)` holds
/// (fresh heap literal/call, or a union-proven local); anything else
/// must keep the probed call (alya-lang/alya#117).
pub fn emit_rc_retain_direct(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X64 => x64::builtins::emit_rc_retain_direct(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_retain_direct(out),
    }
}

/// Duplicate the string in `rax`/`x0` into immortal stable-region memory
/// (B1: named stores must outlive the wrapping ring buffer). Single
/// argument in, same register out. Literals pass through untouched inside
/// the helper (small-value guard); callers skip them statically anyway.
pub fn emit_str_store(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X64 => x64::builtins::emit_str_store(out, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_str_store(out),
    }
}

pub fn emit_rc_release(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
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
        Architecture::X64 => x64::builtins::emit_rc_release_stack(out, offset, stack_offset, os),
        Architecture::ARM64 => arm64::builtins::emit_rc_release_stack(out, offset),
    }
}

/// Probe-free slot release for union-proven rebinds: same shape as
/// `emit_rc_release_stack`, but calls `fn_rc_release_direct`. Call ONLY
/// with a union-proven name (`slot_proven_heap`): the released old
/// value was written by an earlier write to the same name, so the
/// union (every write is null-or-heap) covers it exactly. Offset-only
/// scope cleanups must keep the probed call (stale types possible).
pub fn emit_rc_release_stack_direct(
    out: &mut String,
    arch: Architecture,
    offset: i32,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::X64 => {
            x64::builtins::emit_rc_release_stack_direct(out, offset, stack_offset, os)
        }
        Architecture::ARM64 => arm64::builtins::emit_rc_release_stack_direct(out, offset),
    }
}

pub fn emit_weak_check(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
    lbl: &str,
) {
    match arch {
        Architecture::X64 => x64::builtins::emit_weak_check(out, stack_offset, os, lbl),
        Architecture::ARM64 => arm64::builtins::emit_weak_check(out, lbl),
    }
}
