use super::{arm64, x64, x86};
use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_load_num(out: &mut String, arch: Architecture, val: i64) {
    match arch {
        Architecture::ARM64 => arm64::emit_load_num(out, val),
        Architecture::X64 => x64::emit_load_num(out, val),
        Architecture::X86 => x86::emit_load_num(out, val),
    }
}

pub fn emit_load_float(out: &mut String, arch: Architecture, val: f64) {
    match arch {
        Architecture::ARM64 => arm64::emit_load_float(out, val),
        Architecture::X64 => x64::emit_load_float(out, val),
        Architecture::X86 => x86::emit_load_float(out, val),
    }
}

pub fn emit_int_to_float(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_int_to_float(out),
        Architecture::X64 => x64::emit_int_to_float(out),
        Architecture::X86 => x86::emit_int_to_float(out),
    }
}

pub fn emit_float_to_int(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_float_to_int(out),
        Architecture::X64 => x64::emit_float_to_int(out),
        Architecture::X86 => x86::emit_float_to_int(out),
    }
}

pub fn emit_load_str_label(out: &mut String, arch: Architecture, label: &str, os: OperatingSystem) {
    match arch {
        Architecture::ARM64 => arm64::emit_load_str_label(out, label, os),
        Architecture::X64 => x64::emit_load_str_label(out, label),
        Architecture::X86 => x86::emit_load_str_label(out, label),
    }
}

pub fn emit_load_var(out: &mut String, arch: Architecture, offset: i32, stack_offset: i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_load_var(out, offset, stack_offset),
        Architecture::X64 => x64::emit_load_var(out, offset),
        Architecture::X86 => x86::emit_load_var(out, offset),
    }
}

pub fn emit_load_var_to_scratch(out: &mut String, arch: Architecture, offset: i32, is_float: bool) {
    match arch {
        Architecture::ARM64 => arm64::emit_load_var_to_scratch(out, offset, is_float),
        Architecture::X64 => x64::emit_load_var_to_scratch(out, offset, is_float),
        Architecture::X86 => x86::emit_load_var_to_scratch(out, offset, is_float),
    }
}

pub fn emit_store_var(out: &mut String, arch: Architecture, offset: i32, stack_offset: i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_store_var(out, offset, stack_offset),
        Architecture::X64 => x64::emit_store_var(out, offset),
        Architecture::X86 => x86::emit_store_var(out, offset),
    }
}

pub fn emit_store_var_float(out: &mut String, arch: Architecture, offset: i32, _stack_offset: i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_store_var_float(out, offset),
        Architecture::X64 => x64::emit_store_var_float(out, offset),
        Architecture::X86 => x86::emit_store_var_float(out, offset),
    }
}

pub fn emit_allocate_var(out: &mut String, arch: Architecture, stack_offset: &mut i32) {
    match arch {
        Architecture::ARM64 => arm64::emit_allocate_var(out, stack_offset),
        Architecture::X64 => x64::emit_allocate_var(out, stack_offset),
        Architecture::X86 => x86::emit_allocate_var(out, stack_offset),
    }
}

pub fn emit_push_temp(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_push_temp(out),
        Architecture::X64 => x64::emit_push_temp(out),
        Architecture::X86 => x86::emit_push_temp(out),
    }
}

pub fn emit_pop_temp(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_pop_temp(out),
        Architecture::X64 => x64::emit_pop_temp(out),
        Architecture::X86 => x86::emit_pop_temp(out),
    }
}

pub fn emit_load_global(out: &mut String, arch: Architecture, symbol: &str, os: OperatingSystem) {
    match arch {
        Architecture::ARM64 => arm64::loads::emit_load_global(out, symbol, os),
        Architecture::X64 => x64::loads::emit_load_global(out, symbol),
        Architecture::X86 => x86::loads::emit_load_global(out, symbol),
    }
}

pub fn emit_store_global(out: &mut String, arch: Architecture, symbol: &str, os: OperatingSystem) {
    match arch {
        Architecture::ARM64 => arm64::loads::emit_store_global(out, symbol, os),
        Architecture::X64 => x64::loads::emit_store_global(out, symbol),
        Architecture::X86 => x86::loads::emit_store_global(out, symbol),
    }
}
