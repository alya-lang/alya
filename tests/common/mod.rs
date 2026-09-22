use alya::codegen::{self, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;
pub use std::fs;
pub use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Returns `Some(reason)` when native-execution tests must be skipped on this
/// platform instead of compiling and running binaries. Centralizes the
/// macOS/Darwin gap so every harness reports the same reason instead of
/// silently passing. NOTE: `e2e_*` tests intentionally do NOT consult this —
/// they run wherever a C toolchain exists and act as a canary: if they ever
/// go green on macOS, these skips are stale and must be removed.
pub fn execution_skip_reason() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("Darwin ARM64 target pending full ABI alignment (native execution unverified)")
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn run_alya_code_with_input(source: &str, input: Option<&str>) -> Option<(i32, String)> {
    run_alya_code_with_input_and_args(source, input, &[])
}

#[allow(dead_code)]
pub fn run_alya_code_with_args(source: &str, cli_args: &[&str]) -> Option<(i32, String)> {
    run_alya_code_with_input_and_args(source, None, cli_args)
}

#[allow(dead_code)]
pub fn run_alya_code_with_trace(source: &str) -> Option<(i32, String)> {
    run_alya_code_with_options(source, None, &[], true)
}

pub fn run_alya_code_with_input_and_args(
    source: &str,
    input: Option<&str>,
    cli_args: &[&str],
) -> Option<(i32, String)> {
    run_alya_code_with_options(source, input, cli_args, false)
}

pub fn run_alya_code_with_options(
    source: &str,
    input: Option<&str>,
    cli_args: &[&str],
    mem_trace: bool,
) -> Option<(i32, String)> {
    // Check if gcc is available
    if Command::new("gcc").arg("--version").output().is_err() {
        eprintln!("Skipping E2E test: GCC is not available in PATH.");
        return None;
    }

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parser error");
    alya::parser::resolve_imports(&mut ast, std::path::Path::new("."))
        .expect("Module import resolution failed");

    let os = if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    };

    let arch = if cfg!(target_arch = "aarch64") {
        Architecture::ARM64
    } else if cfg!(target_arch = "x86") {
        Architecture::X86
    } else {
        Architecture::X64
    };

    let asm_code = if mem_trace {
        codegen::generate_full(&ast, arch, os, false, true).0
    } else {
        codegen::generate(&ast, arch, os)
    };

    let pid = std::process::id();
    let id = TEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let asm_path = format!("temp_e2e_{}_{}_{}.s", pid, id, time);
    let exe_path = if cfg!(target_os = "windows") {
        format!("temp_e2e_{}_{}_{}.exe", pid, id, time)
    } else {
        format!("temp_e2e_{}_{}_{}", pid, id, time)
    };

    fs::write(&asm_path, &asm_code).expect("Failed to write temp asm file");

    let mut gcc = Command::new("gcc");
    gcc.arg(&asm_path).arg("-o").arg(&exe_path);
    if matches!(arch, Architecture::X86) {
        gcc.arg("-m32");
    }
    if matches!(os, OperatingSystem::Linux) {
        gcc.arg("-no-pie");
        gcc.arg("-lm");
    }
    if matches!(os, OperatingSystem::Windows) {
        gcc.arg("-lws2_32");
    }

    if matches!(os, OperatingSystem::MacOS) {
        gcc.arg("-Wl,-dead_strip");
    } else {
        gcc.arg("-Wl,--gc-sections");
    }

    gcc.arg("-L.");
    for lib in codegen::collect_extern_libraries(&ast) {
        gcc.arg(format!("-l{}", lib));
    }

    let gcc_out = gcc.output().expect("GCC invocation failed");
    let _ = fs::remove_file(&asm_path);

    if !gcc_out.status.success() {
        let _ = fs::remove_file(&exe_path);
        panic!(
            "GCC compilation error:\n{}",
            String::from_utf8_lossy(&gcc_out.stderr)
        );
    }

    let run_cmd = if cfg!(target_os = "windows") {
        format!(".\\{}", exe_path)
    } else {
        format!("./{}", exe_path)
    };

    let mut cmd = Command::new(&run_cmd);
    cmd.args(cli_args);
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Program execution failed");

    if let Some(in_str) = input {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(in_str.as_bytes());
        }
    }

    let prog_out = child
        .wait_with_output()
        .expect("Failed to wait on child process");
    let _ = fs::remove_file(&exe_path);

    let code = prog_out.status.code().unwrap_or(-1);
    let mut output = String::from_utf8_lossy(&prog_out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&prog_out.stderr).replace("\r\n", "\n");
    output.push_str(&stderr);

    Some((code, output))
}

#[allow(dead_code)]
pub fn run_alya_code_full(source: &str) -> Option<(i32, String)> {
    run_alya_code_with_input(source, None)
}

#[allow(dead_code)]
pub fn run_alya_code(source: &str) -> Option<String> {
    run_alya_code_full(source).map(|(_, out)| out)
}
