use crate::codegen::{Architecture, OperatingSystem};
use std::fs;
use std::process::Command;

pub fn compile_with_gcc(
    asm_file: &str,
    exe_file: &str,
    arch: Architecture,
    os: OperatingSystem,
    extra_libs: &[String],
    c_objects: &[std::path::PathBuf],
    extra_link_args: &[String],
) -> Result<(), String> {
    let mut gcc_args = vec![asm_file.to_string(), "-o".to_string(), exe_file.to_string()];

    for obj in c_objects {
        gcc_args.push(crate::driver::c_builder::path_to_gcc_arg(obj));
    }

    if matches!(os, OperatingSystem::Linux) {
        gcc_args.push("-no-pie".to_string());
        gcc_args.push("-lm".to_string());
        gcc_args.push("-lpthread".to_string());
    }

    if matches!(os, OperatingSystem::Windows) {
        gcc_args.push("-lws2_32".to_string());
    }

    // Dead Code Elimination at linker level: discard unused sections
    if matches!(os, OperatingSystem::MacOS) {
        gcc_args.push("-Wl,-dead_strip".to_string());
    } else {
        gcc_args.push("-Wl,--gc-sections".to_string());
    }

    gcc_args.push("-L.".to_string());
    for lib in extra_libs {
        gcc_args.push(format!("-l{}", lib));
    }
    for arg in extra_link_args {
        gcc_args.push(arg.clone());
    }

    let toolchain = crate::driver::toolchain::resolve_toolchain(arch, os, false)?;
    let gcc_result = Command::new(&toolchain.compiler_path)
        .args(&gcc_args)
        .output();

    let _ = fs::remove_file(asm_file);

    match gcc_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let hint = if matches!(os, OperatingSystem::MacOS) && !cfg!(target_os = "macos") {
                    "\nNote: Linking a native macOS Mach-O binary requires macOS (clang/gcc) or an Apple cross-compilation toolchain."
                } else {
                    ""
                };
                Err(format!(
                    "Compiler ({}) invocation failed:\n{}{}",
                    toolchain.compiler_path.display(),
                    stderr,
                    hint
                ))
            } else {
                // On macOS ARM64 (Apple Silicon), automatically ad-hoc codesign binary
                if matches!(os, OperatingSystem::MacOS)
                    && matches!(arch, Architecture::ARM64)
                    && cfg!(target_os = "macos")
                {
                    let _ = Command::new("codesign")
                        .args(["-s", "-", "-f", exe_file])
                        .output();
                }
                Ok(())
            }
        }
        Err(e) => Err(format!(
            "Error: Failed to execute compiler '{}': {}\nMake sure the toolchain is installed properly (run 'alya toolchain status').",
            toolchain.compiler_path.display(),
            e
        )),
    }
}

pub fn execute_binary(
    exe_file: &str,
    run_args: &[String],
    delete_after: bool,
) -> Result<(), String> {
    let run_path = if cfg!(target_os = "windows") {
        format!(".\\{}", exe_file)
    } else {
        format!("./{}", exe_file)
    };

    let mut cmd = Command::new(&run_path);
    cmd.args(run_args);
    let mut child = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Error: Failed to execute '{}': {}", run_path, e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Execution error: {}", e))?;

    if delete_after {
        let _ = fs::remove_file(exe_file);
    }

    if !status.success() {
        let code = status.code().unwrap_or(1);
        // A Unix child killed by a signal reports no exit code; without this
        // branch that death is indistinguishable from a silent `exit(1)`
        // (e.g. the tensor macOS ARM64 bench abort). Report it explicitly.
        #[cfg(unix)]
        if status.code().is_none() {
            use std::os::unix::process::ExitStatusExt;
            match status.signal() {
                Some(sig) => eprintln!(
                    "Error: program terminated by signal {} ({})",
                    sig,
                    signal_name(sig)
                ),
                None => eprintln!("Error: program terminated abnormally"),
            }
        }
        // Windows has no signals: a child killed by the OS reports the
        // NTSTATUS as its exit code. Name the known crash codes so an
        // unhandled fault (one the in-binary handler passed along) is not
        // silent. Mirrors tools/test_runner.rs crash decoding.
        #[cfg(windows)]
        match code as u32 {
            0xC0000005 => eprintln!("Error: program terminated by Access Violation (0xC0000005)"),
            0xC00000FD => eprintln!("Error: program terminated by Stack Overflow (0xC00000FD)"),
            0xC000001D => {
                eprintln!("Error: program terminated by Illegal Instruction (0xC000001D)")
            }
            0xC0000094 => {
                eprintln!("Error: program terminated by Integer Divide by Zero (0xC0000094)")
            }
            0xC0000409 => {
                eprintln!("Error: program terminated by Stack Buffer Overrun (0xC0000409)")
            }
            0x80000003 => eprintln!("Error: program terminated by Breakpoint Trap (0x80000003)"),
            0xC000000D => {
                eprintln!("Error: program terminated by Invalid Parameter (0xC000000D)")
            }
            // Catch-all for unlisted NTSTATUS crashes so a new fault is
            // reported with its code instead of exiting silently.
            c if c >= 0x80000000 => {
                eprintln!(
                    "Error: program terminated by OS exception (NTSTATUS: 0x{:08X})",
                    c
                )
            }
            _ => {}
        }
        std::process::exit(code);
    }

    Ok(())
}

#[cfg(unix)]
fn signal_name(sig: i32) -> &'static str {
    match sig {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        // SIGBUS is 7 on Linux but 10 on macOS (where 7 is SIGEMT and
        // 10 is SIGUSR1 on Linux). Mirrors codegen/runtime crash handler.
        #[cfg(target_os = "macos")]
        7 => "SIGEMT",
        #[cfg(not(target_os = "macos"))]
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        #[cfg(target_os = "macos")]
        10 => "SIGBUS",
        #[cfg(not(target_os = "macos"))]
        10 => "SIGUSR1",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => "unknown signal",
    }
}
