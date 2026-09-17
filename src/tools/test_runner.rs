use crate::codegen::{self, Architecture, OperatingSystem};
use crate::driver::runner;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

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
    let tests_subdir = path.join("tests");
    let target_dir = if tests_subdir.is_dir() {
        &tests_subdir
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
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_asm = format!("temp_test_{}_{}.s", pid, test_id);
    let temp_exe = if matches!(os, OperatingSystem::Windows) {
        format!("temp_test_{}_{}.exe", pid, test_id)
    } else {
        format!("temp_test_{}_{}", pid, test_id)
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

    // 6. Execute binary with timeout
    let exe_path = if matches!(os, OperatingSystem::Windows) {
        format!(".\\{}", temp_exe)
    } else {
        format!("./{}", temp_exe)
    };

    let mut child = Command::new(&exe_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute test binary '{}': {}", exe_path, e))?;

    let start_wait = Instant::now();
    let timeout_limit = std::time::Duration::from_secs(30);
    let mut exited = false;
    let mut exit_status = None;

    while start_wait.elapsed() < timeout_limit {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("Failed to check status of '{}': {}", exe_path, e))?
        {
            exit_status = Some(status);
            exited = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let (stdout_bytes, stderr_bytes, is_timeout) = if exited {
        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to read output of '{}': {}", exe_path, e))?;
        (output.stdout, output.stderr, false)
    } else {
        let _ = child.kill();
        let output = child
            .wait_with_output()
            .unwrap_or_else(|_| std::process::Output {
                status: exit_status.unwrap_or_default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        (output.stdout, output.stderr, true)
    };

    let _ = fs::remove_file(&temp_exe);
    let elapsed = start_time.elapsed().as_millis();

    if is_timeout {
        return Ok((
            false,
            format!(
                "Test timed out after 30 seconds.\nStdout:\n{}\nStderr:\n{}",
                String::from_utf8_lossy(&stdout_bytes),
                String::from_utf8_lossy(&stderr_bytes)
            ),
            elapsed,
        ));
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
    let full_out = format!("{}{}", stdout, stderr);

    let success = exit_status.map(|s| s.success()).unwrap_or(false)
        && !full_out.contains("[FAIL]")
        && !full_out.contains("Runtime error:");
    Ok((success, full_out, elapsed))
}

struct TestResultItem {
    display_name: String,
    outcome: Result<(bool, String, u128), String>,
}

fn handle_test_result(
    display_name: &str,
    outcome: Result<(bool, String, u128), String>,
    passed: &mut usize,
    failed: &mut usize,
) {
    match outcome {
        Ok((true, _out, ms)) => {
            *passed += 1;
            println!("  \x1b[1;32m✓\x1b[0m {:<40} ({:>4} ms)", display_name, ms);
        }
        Ok((false, out, ms)) => {
            *failed += 1;
            println!(
                "  \x1b[1;31m✗\x1b[0m {:<40} ({:>4} ms) - FAILED",
                display_name, ms
            );
            if out.trim().is_empty() {
                println!("      \x1b[91m| (Process terminated abnormally with no output)\x1b[0m");
            } else {
                for line in out.lines().take(8) {
                    println!("      \x1b[90m|\x1b[0m {}", line);
                }
            }
        }
        Err(err) => {
            *failed += 1;
            println!("  \x1b[1;31m✗\x1b[0m {:<40} - ERROR", display_name);
            println!("      \x1b[91m{}\x1b[0m", err);
        }
    }
}

/// Runs the Alya test suite on the specified path with optional parallel worker jobs.
pub fn run_tests(
    path_str: &str,
    arch: Architecture,
    os: OperatingSystem,
    jobs: Option<usize>,
) -> Result<(), String> {
    let root = Path::new(path_str);
    let test_files = discover_test_files(root);

    if test_files.is_empty() {
        println!("No test files found in '{}'.", path_str);
        return Ok(());
    }

    let default_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let max_default = if cfg!(target_os = "windows") { 4 } else { 8 };
    let num_workers = jobs
        .unwrap_or_else(|| default_threads.min(max_default))
        .max(1)
        .min(test_files.len());

    println!("\n=== Running Alya Test Suite ===");
    if num_workers == 1 {
        println!(
            "Discovered {} test file(s) in '{}' (sequential)\n",
            test_files.len(),
            path_str
        );
    } else {
        println!(
            "Discovered {} test file(s) in '{}' (parallel, {} workers)\n",
            test_files.len(),
            path_str,
            num_workers
        );
    }

    let mut passed = 0;
    let mut failed = 0;
    let total_start = Instant::now();

    if num_workers == 1 {
        for file in &test_files {
            let display_name = file.display().to_string();
            let outcome = execute_test_file(file, arch, os);
            handle_test_result(&display_name, outcome, &mut passed, &mut failed);
        }
    } else {
        let (tx, rx) = mpsc::channel();
        let queue = Arc::new(Mutex::new(test_files.into_iter().collect::<VecDeque<_>>()));

        let mut handles = Vec::new();
        for _ in 0..num_workers {
            let q = Arc::clone(&queue);
            let sender = tx.clone();
            handles.push(thread::spawn(move || loop {
                let file = {
                    let mut locked = q.lock().unwrap();
                    locked.pop_front()
                };
                match file {
                    Some(path) => {
                        let display_name = path.display().to_string();
                        let outcome = execute_test_file(&path, arch, os);
                        let _ = sender.send(TestResultItem {
                            display_name,
                            outcome,
                        });
                    }
                    None => break,
                }
            }));
        }
        drop(tx);

        while let Ok(item) = rx.recv() {
            handle_test_result(&item.display_name, item.outcome, &mut passed, &mut failed);
        }

        for h in handles {
            let _ = h.join();
        }
    }

    let total_time = total_start.elapsed().as_millis();
    println!("\n----------------------------------------");
    let mode_str = if num_workers == 1 {
        "sequential".to_string()
    } else {
        format!("parallel, {} workers", num_workers)
    };
    if failed == 0 {
        println!(
            "\x1b[1;32m✓ Test Results: {} passed, 0 failed in {} ms ({})\x1b[0m",
            passed, total_time, mode_str
        );
        println!("----------------------------------------\n");
        Ok(())
    } else {
        println!(
            "\x1b[1;31m✗ Test Results: {} passed, {} failed in {} ms ({})\x1b[0m",
            passed, failed, total_time, mode_str
        );
        println!("----------------------------------------\n");
        Err(format!("Test suite completed with {} failure(s).", failed))
    }
}
