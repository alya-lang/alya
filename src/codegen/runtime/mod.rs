#[rustfmt::skip]
pub mod arm64;
pub mod data;
#[rustfmt::skip]
pub mod x64;
#[rustfmt::skip]
pub mod x86;

use super::target::{Architecture, OperatingSystem};

use crate::codegen::context::StructDefInfo;
use std::collections::HashMap;

pub fn emit_runtime(
    out: &mut String,
    arch: Architecture,
    os: OperatingSystem,
    structs: &HashMap<String, StructDefInfo>,
    interfaces: &HashMap<String, crate::codegen::context::InterfaceDefInfo>,
    vtables: &HashMap<(String, String), String>,
) {
    data::emit_data_sections(out, arch, os, structs, interfaces, vtables);

    out.push_str("\n.text\n");

    match arch {
        Architecture::ARM64 => arm64::emit_arm64_runtime(out, os),
        Architecture::X64 => x64::emit_x64_runtime(out, os),
        Architecture::X86 => x86::emit_x86_runtime(out, os),
    }
}
