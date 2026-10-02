pub mod arm64;
pub mod builtins;
pub mod control;
pub mod loads;
pub mod ops;
pub mod say;
pub mod x64;

pub use builtins::*;
pub use control::*;
pub use loads::*;
pub use ops::*;
pub use say::*;

use crate::codegen::target::{Architecture, OperatingSystem};

pub fn emit_header(out: &mut String, arch: Architecture, os: OperatingSystem) {
    match arch {
        Architecture::ARM64 => arm64::emit_header(out, os),
        Architecture::X64 => x64::emit_header(out, os),
    }
}

pub fn emit_footer(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_footer(out),
        Architecture::X64 => x64::emit_footer(out),
    }
}
