use crate::codegen::{Architecture, OperatingSystem};
use std::fs;
use std::process::Command;

/// Pre-assemble duplicate-symbol check (alya-lang/alya#153).
///
/// A top-level function sharing its mangled label with a runtime builtin
/// (e.g. user `function get` vs runtime `fn_get`) used to surface as a
/// raw assembler error (`symbol 'fn_get' is already defined`). The
/// assembler only sees the flat labels, so catch repeats here — where
/// every definition is known — and report which symbol collided instead.
///
/// Only file-scope `name:` definitions are considered. Local labels
/// (`.L<counter>`, fresh per site) and directives (`.global`, `.seh_proc`,
/// `.extern`, all dot-led) cannot collide by construction. A single
/// leading underscore is normalized: macOS spells every symbol `_name`
/// while other targets spell it `name`.
pub fn check_duplicate_symbols(asm: &str) -> Result<(), String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut dups: Vec<String> = Vec::new();
    for line in asm.lines() {
        let t = line.trim_start();
        let mut chars = t.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => continue,
        }
        let end = t
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'))
            .unwrap_or(t.len());
        let (name, rest) = t.split_at(end);
        if !rest.starts_with(':') {
            continue;
        }
        let key = name.strip_prefix('_').unwrap_or(name);
        if !seen.insert(key.to_string()) && !dups.iter().any(|d| d == key) {
            dups.push(key.to_string());
        }
    }
    if dups.is_empty() {
        return Ok(());
    }
    let mut msg = String::from(
        "Error: duplicate symbol(s) defined more than once in the generated assembly:",
    );
    for d in &dups {
        msg.push_str(&format!("\n  `{}`", d));
        if let Some(bare) = d.strip_prefix("fn_") {
            msg.push_str(&format!(
                " — a function named `{}` collides with a runtime builtin of the same name; rename the function",
                bare
            ));
        }
    }
    Err(msg)
}

/// Max seconds for one gcc assemble+link invocation, overridable via
/// `ALYA_GCC_TIMEOUT_SECS`. Link-phase stalls (AV locks on fresh
/// executables, pathological inputs) otherwise hang the caller forever:
/// unlike suite *execution* (test_runner's `suite_timeout`), the compile
/// step had no bound, stalling whole `alya test` runs with no summary
/// (the stuck suite never reports). Absent or invalid values fall back
/// to 300; values below 1 clamp to 1.
fn gcc_timeout() -> (u64, std::time::Duration) {
    let secs: u64 = std::env::var("ALYA_GCC_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(300);
    (secs, std::time::Duration::from_secs(secs))
}

pub fn compile_with_gcc(
    asm_file: &str,
    exe_file: &str,
    arch: Architecture,
    os: OperatingSystem,
    extra_libs: &[String],
    c_objects: &[std::path::PathBuf],
    extra_link_args: &[String],
) -> Result<(), String> {
    // Duplicate labels (e.g. a function colliding with a runtime builtin,
    // alya-lang/alya#153) fail here with a named diagnostic instead of a
    // raw assembler error. The driver `run` path checks earlier (covering
    // `-S` output); this covers `test`/`bench`, which assemble separately.
    if let Ok(text) = fs::read_to_string(asm_file) {
        if let Err(e) = check_duplicate_symbols(&text) {
            let _ = fs::remove_file(asm_file);
            return Err(e);
        }
    }
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
    // Bounded wait (see `gcc_timeout`): a stuck gcc/ld must fail loudly
    // instead of hanging the caller with no output.
    let mut child = Command::new(&toolchain.compiler_path)
        .args(&gcc_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = fs::remove_file(asm_file);
            format!(
                "Error: Failed to execute compiler '{}': {}\nMake sure the toolchain is installed properly (run 'alya toolchain status').",
                toolchain.compiler_path.display(),
                e
            )
        })?;
    let (limit_secs, limit) = gcc_timeout();
    let start = std::time::Instant::now();
    let mut exited = false;
    while start.elapsed() < limit {
        match child.try_wait().map_err(|e| {
            format!(
                "Error: Failed to check status of compiler '{}': {}",
                toolchain.compiler_path.display(),
                e
            )
        })? {
            Some(_) => {
                exited = true;
                break;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }
    // On timeout kill and reap without draining pipes (a surviving
    // grandchild holding them must not block us the way it would in
    // `wait_with_output`).
    let gcc_result = if exited {
        child.wait_with_output().map_err(|e| e.to_string())
    } else {
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_file(asm_file);
        return Err(format!(
            "Compiler ({}) timed out after {}s linking '{}' (override via ALYA_GCC_TIMEOUT_SECS).",
            toolchain.compiler_path.display(),
            limit_secs,
            exe_file
        ));
    };

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

#[cfg(test)]
mod tests {
    use super::check_duplicate_symbols;

    #[test]
    fn user_runtime_symbol_collision_is_reported() {
        // alya-lang/alya#153: user `function get` vs runtime `fn_get`.
        let asm = "    .global fn_get\nfn_get:\n    ret\nfn_other:\n    ret\nfn_get:\n    ret\n";
        let err = check_duplicate_symbols(asm).unwrap_err();
        assert!(err.contains("`fn_get`"), "got: {}", err);
        assert!(err.contains("`get`"), "got: {}", err);
        assert!(err.contains("runtime builtin"), "got: {}", err);
    }

    #[test]
    fn clean_assembly_passes_and_locals_are_ignored() {
        let asm =
            "    .global fn_main\nfn_main:\n    call fn_get\n.L1:\n    jmp .L1\nmain:\n    ret\n";
        assert!(check_duplicate_symbols(asm).is_ok());
    }

    #[test]
    fn macos_underscore_spelling_matches() {
        // `_fn_get:` (macOS) and `fn_get:` (same file) are one symbol.
        let asm = "_fn_get:\n    ret\nfn_get:\n    ret\n";
        let err = check_duplicate_symbols(asm).unwrap_err();
        assert!(err.contains("`fn_get`"), "got: {}", err);
    }
}
