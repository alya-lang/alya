use crate::codegen::{self, Architecture, OperatingSystem};
use crate::driver::runner;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Compiles a source snippet to a temporary executable binary using GCC.
/// Returns the name of the executable file (or empty string if source was empty).
pub fn compile_snippet_to_temp_exe(
    source: &str,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<String, String> {
    if source.trim().is_empty() {
        return Ok(String::new());
    }

    // 1. Lexer
    let mut lexer = Lexer::new(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer error: {}", e))?;

    // 2. Parser
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().map_err(|e| format!("Parser error: {}", e))?;

    // 3. Module Resolution
    let base_dir = Path::new(".");
    crate::parser::resolve_imports(&mut ast, base_dir)
        .map_err(|e| format!("Import error: {}", e))?;

    // 4. Codegen
    let asm_code = codegen::generate(&ast, arch, os);
    let extra_libs = codegen::collect_extern_libraries(&ast);

    // 5. Compile with GCC to temp executable
    let pid = std::process::id();
    let rand_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        % 1_000_000_000;
    let temp_asm = format!("temp_repl_{}_{}.s", pid, rand_id);
    let temp_exe = if matches!(os, OperatingSystem::Windows) {
        format!("temp_repl_{}_{}.exe", pid, rand_id)
    } else {
        format!("temp_repl_{}_{}", pid, rand_id)
    };

    fs::write(&temp_asm, &asm_code)
        .map_err(|e| format!("Failed to write temporary assembly: {}", e))?;

    let gcc_res = runner::compile_with_gcc(&temp_asm, &temp_exe, arch, os, &extra_libs, &[], &[]);
    let _ = fs::remove_file(&temp_asm);

    gcc_res?;

    Ok(temp_exe)
}

/// Compiles and runs a source snippet, capturing stdout and stderr.
pub fn execute_code_snippet(
    source: &str,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<(bool, String, String), String> {
    let temp_exe = compile_snippet_to_temp_exe(source, arch, os)?;
    if temp_exe.is_empty() {
        return Ok((true, String::new(), String::new()));
    }

    let exe_path = if matches!(os, OperatingSystem::Windows) {
        format!(".\\{}", temp_exe)
    } else {
        format!("./{}", temp_exe)
    };

    let run_res = Command::new(&exe_path)
        .stdin(std::process::Stdio::inherit())
        .output();

    let _ = fs::remove_file(&temp_exe);

    match run_res {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok((output.status.success(), stdout, stderr))
        }
        Err(e) => Err(format!("Execution failed: {}", e)),
    }
}

/// Compiles and runs a source snippet interactively, inheriting stdin, stdout, and stderr.
pub fn execute_code_snippet_interactive(
    source: &str,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<bool, String> {
    let temp_exe = compile_snippet_to_temp_exe(source, arch, os)?;
    if temp_exe.is_empty() {
        return Ok(true);
    }

    let exe_path = if matches!(os, OperatingSystem::Windows) {
        format!(".\\{}", temp_exe)
    } else {
        format!("./{}", temp_exe)
    };

    let status = Command::new(&exe_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    let _ = fs::remove_file(&temp_exe);

    match status {
        Ok(s) => Ok(s.success()),
        Err(e) => Err(format!("Execution failed: {}", e)),
    }
}
