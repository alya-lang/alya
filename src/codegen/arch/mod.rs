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

pub fn emit_header(out: &mut String, arch: Architecture, os: OperatingSystem, no_std: bool) {
    match arch {
        Architecture::ARM64 => arm64::emit_header(out, os, no_std),
        Architecture::X64 => x64::emit_header(out, os, no_std),
    }
}

pub fn emit_footer(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_footer(out),
        Architecture::X64 => x64::emit_footer(out),
    }
}

/// Minimal entry/exit pair for `--no-std` (alya-lang/alya#191): no
/// extern decls, no argc/argv stores, no console/socket init. The exit
/// preserves the return register (the user's `main` return becomes the
/// process exit code) instead of zeroing it like the hosted footer.
pub fn emit_footer_nostd(out: &mut String, arch: Architecture) {
    match arch {
        Architecture::ARM64 => arm64::emit_footer_nostd(out),
        Architecture::X64 => x64::emit_footer_nostd(out),
    }
}
