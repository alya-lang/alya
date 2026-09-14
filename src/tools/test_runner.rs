use crate::codegen::{self, Architecture, OperatingSystem};
use crate::driver::runner;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Discovers test files in the specified path.
pub fn discover_test_files(path: &Path) -> Vec<PathBuf> {
    let mut tests = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("alya") {
            tests.push(path.to_path_buf());
        }
        return tests;
    }

    // If path is a directory, check tests/ directory or search for test_*.alya / *_test.alya
    let target_dir = if path == Path::new(".") && Path::new("tests").is_dir() {
        Path::new("tests")
    } else {
        path
    };

    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.starts_with('.') || file_name == "target" || file_name == "build" {
                continue;
            }
            if p.is_dir() {
                tests.extend(discover_test_files(&p));
            } else if p.extension().and_then(|e| e.to_str()) == Some("alya") {
                tests.push(p);
            }
        }
    }

    tests.sort();
    tests
}

/// Compiles and runs an Alya test file, returning (success, stdout, duration_ms).
pub fn execute_test_file(
    path: &Path,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<(bool, String, u128), String> {
    let start_time = Instant::now();
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file '{}': {}", path.display(), e))?;

    // 1. Lexer
    let mut lexer = Lexer::new(&source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer error in '{}': {}", path.display(), e))?;

    // 2. Parser
    let mut parser = Parser::new(tokens);
    let mut ast = parser
        .parse()
        .map_err(|e| format!("Parser error in '{}': {}", path.display(), e))?;

    // 3. Module Resolution
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let imported_files = crate::parser::resolve_imports_with_sources(&mut ast, base_dir)
        .map_err(|e| format!("Import resolution error in '{}': {}", path.display(), e))?;

    // 4. Codegen
    let asm_code = codegen::generate(&ast, arch, os);

    // 5. Compile with GCC to temp executable
    let pid = std::process::id();
    let rand_id = (start_time.elapsed().as_nanos() % 100000) as u32;
    let temp_asm = format!("temp_test_{}_{}.s", pid, rand_id);
    let temp_exe = if matches!(os, OperatingSystem::Windows) {
        format!("temp_test_{}_{}.exe", pid, rand_id)
    } else {
        format!("temp_test_{}_{}", pid, rand_id)
    };

    fs::write(&temp_asm, &asm_code)
        .map_err(|e| format!("Failed to write temporary assembly: {}", e))?;

    let c_plan = crate::driver::c_builder::discover_c_build_plan(path, &imported_files)?;
    let c_objects = crate::driver::c_builder::build_c_objects(&c_plan, arch, os)?;
    let mut extra_libs = codegen::collect_extern_libraries(&ast);
    extra_libs.retain(|lib| !c_plan.provided_libs.contains(lib));

    let gcc_res = runner::compile_with_gcc(&temp_asm, &temp_exe, arch, os, &extra_libs, &c_objects);
    let _ = fs::remove_file(&temp_asm);
    if let Err(err) = gcc_res {
        return Err(format!(
            "GCC compilation failed for '{}': {}",
            path.display(),
            err
        ));
    }

    // 6. Execute binary
    let exe_path = if matches!(os, OperatingSystem::Windows) {
        format!(".\\{}", temp_exe)
    } else {
        format!("./{}", temp_exe)
    };

    let run_res = Command::new(&exe_path).output();

    let _ = fs::remove_file(&temp_exe);
    let elapsed = start_time.elapsed().as_millis();

    match run_res {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let full_out = format!("{}{}", stdout, stderr);

            let success = output.status.success()
                && !full_out.contains("[FAIL]")
                && !full_out.contains("Runtime error:");
            Ok((success, full_out, elapsed))
        }
        Err(e) => Err(format!(
            "Failed to execute test binary '{}': {}",
            exe_path, e
        )),
    }
}

/// Runs the Alya test suite on the specified path.
pub fn run_tests(path_str: &str, arch: Architecture, os: OperatingSystem) -> Result<(), String> {
    let root = Path::new(path_str);
    let test_files = discover_test_files(root);

    if test_files.is_empty() {
        println!("No test files found in '{}'.", path_str);
        return Ok(());
    }

    println!("\n=== Running Alya Test Suite ===");
    println!(
        "Discovered {} test file(s) in '{}'\n",
        test_files.len(),
        path_str
    );

    let mut passed = 0;
    let mut failed = 0;
    let total_start = Instant::now();

    for file in &test_files {
        let display_name = file.display().to_string();
        match execute_test_file(file, arch, os) {
            Ok((true, _out, ms)) => {
                passed += 1;
                println!("  \x1b[1;32m✓\x1b[0m {:<40} ({:>4} ms)", display_name, ms);
            }
            Ok((false, out, ms)) => {
                failed += 1;
                println!(
                    "  \x1b[1;31m✗\x1b[0m {:<40} ({:>4} ms) - FAILED",
                    display_name, ms
                );
                if out.trim().is_empty() {
                    println!(
                        "      \x1b[91m| (Process terminated abnormally with no output)\x1b[0m"
                    );
                } else {
                    for line in out.lines().take(8) {
                        println!("      \x1b[90m|\x1b[0m {}", line);
                    }
                }
            }
            Err(err) => {
                failed += 1;
                println!("  \x1b[1;31m✗\x1b[0m {:<40} - ERROR", display_name);
                println!("      \x1b[91m{}\x1b[0m", err);
            }
        }
    }

    let total_time = total_start.elapsed().as_millis();
    println!("\n----------------------------------------");
    if failed == 0 {
        println!(
            "\x1b[1;32m✓ Test Results: {} passed, 0 failed in {} ms\x1b[0m",
            passed, total_time
        );
        println!("----------------------------------------\n");
        Ok(())
    } else {
        println!(
            "\x1b[1;31m✗ Test Results: {} passed, {} failed in {} ms\x1b[0m",
            passed, failed, total_time
        );
        println!("----------------------------------------\n");
        Err(format!("Test suite completed with {} failure(s).", failed))
    }
}
