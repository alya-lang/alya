use super::{arm64, x64, x86};
use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_say_str(
    out: &mut String,
    arch: Architecture,
    label: &str,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_str(out, label, fmt_label, os),
        Architecture::X64 => x64::emit_say_str(out, label, fmt_label, stack_offset, os),
        Architecture::X86 => x86::emit_say_str(out, label, fmt_label),
    }
}

pub fn emit_say_str_lit(
    out: &mut String,
    arch: Architecture,
    label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_str_lit(out, label, os),
        Architecture::X64 => x64::emit_say_str_lit(out, label, stack_offset, os),
        Architecture::X86 => x86::emit_say_str_lit(out, label),
    }
}

pub fn emit_say_offset(
    out: &mut String,
    arch: Architecture,
    offset: i32,
    stack_offset: i32,
    fmt_label: &str,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_offset(out, offset, stack_offset, fmt_label, os),
        Architecture::X64 => x64::emit_say_offset(out, offset, fmt_label, stack_offset, os),
        Architecture::X86 => x86::emit_say_offset(out, offset, fmt_label),
    }
}

pub fn emit_say_num_const(
    out: &mut String,
    arch: Architecture,
    val: i64,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_num_const(out, val, fmt_label, os),
        Architecture::X64 => x64::emit_say_num_const(out, val, fmt_label, stack_offset, os),
        Architecture::X86 => x86::emit_say_num_const(out, val, fmt_label),
    }
}

pub fn emit_say_acc(
    out: &mut String,
    arch: Architecture,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_acc(out, fmt_label, os),
        Architecture::X64 => x64::emit_say_acc(out, fmt_label, stack_offset, os),
        Architecture::X86 => x86::emit_say_acc(out, fmt_label),
    }
}

pub fn emit_say_float(
    out: &mut String,
    arch: Architecture,
    fmt_label: &str,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_say_float(out, fmt_label, os),
        Architecture::X64 => x64::emit_say_float(out, fmt_label, stack_offset, os),
        Architecture::X86 => x86::emit_say_float(out, fmt_label),
    }
}

pub fn emit_say_interpolated(
    out: &mut String,
    arch: Architecture,
    fmt_label: &str,
    is_floats: &[bool],
    is_strings: &[bool],
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => {
            arm64::emit_say_interpolated_pop_and_call(out, fmt_label, is_floats, os)
        }
        Architecture::X64 => {
            x64::emit_say_interpolated_pop_and_call(out, fmt_label, is_floats, stack_offset, os)
        }
        Architecture::X86 => {
            let mut args_bytes = 0usize;
            for (i, f) in is_floats.iter().enumerate() {
                if *f || !is_strings.get(i).copied().unwrap_or(false) {
                    args_bytes += 8;
                } else {
                    args_bytes += 4;
                }
            }
            x86::emit_say_interpolated_call(out, fmt_label, args_bytes)
        }
    }
}

pub fn emit_string_concat_call(
    out: &mut String,
    arch: Architecture,
    stack_offset: i32,
    os: OperatingSystem,
) {
    match arch {
        Architecture::ARM64 => arm64::emit_string_concat_call(out),
        Architecture::X64 => x64::emit_string_concat_call(out, stack_offset, os),
        Architecture::X86 => x86::emit_string_concat_call(out),
    }
}
