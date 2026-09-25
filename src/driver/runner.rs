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

    if matches!(arch, Architecture::X86) {
        gcc_args.insert(0, "-m32".to_string());
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
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGBUS",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => "unknown signal",
    }
}
