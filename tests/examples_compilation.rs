mod common;
use common::expected::get_expected_output;

use alya::codegen::{self, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;
use std::fs;

#[test]
fn test_all_examples_compile_to_assembly() {
    let mut examples: Vec<String> = fs::read_dir("examples")
        .expect("Failed to read examples directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("alya") {
                path.file_name()?.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    examples.sort();
    assert!(!examples.is_empty(), "No .alya files found in examples/");

    for example_name in examples {
        let path = format!("examples/{}", example_name);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read example file '{}': {}", path, e));

        // 1. Lexer
        let mut lexer = Lexer::new(&source);
        let tokens = lexer
            .tokenize()
            .unwrap_or_else(|e| panic!("Lexer failed for '{}': {}", example_name, e));

        // 2. Parser
        let mut parser = Parser::new(tokens);
        let mut ast = parser
            .parse()
            .unwrap_or_else(|e| panic!("Parser failed for '{}': {}", example_name, e));

        let base_dir = std::path::Path::new(&path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        alya::parser::resolve_imports(&mut ast, base_dir)
            .unwrap_or_else(|e| panic!("Import resolution failed for '{}': {}", example_name, e));

        // 3. Codegen for x64
        let x64_asm = codegen::generate(&ast, Architecture::X64, OperatingSystem::Windows);
        assert!(
            !x64_asm.is_empty(),
            "Empty x64 assembly generated for '{}'",
            example_name
        );
        assert!(
            x64_asm.contains(".global main"),
            "Missing main entry in x64 for '{}'",
            example_name
        );

        // 4. Codegen for x86
        let x86_asm = codegen::generate(&ast, Architecture::X86, OperatingSystem::Linux);
        assert!(
            !x86_asm.is_empty(),
            "Empty x86 assembly generated for '{}'",
            example_name
        );

        // 5. Codegen for arm64
        let arm64_asm = codegen::generate(&ast, Architecture::ARM64, OperatingSystem::Linux);
        assert!(
            !arm64_asm.is_empty(),
            "Empty arm64 assembly generated for '{}'",
            example_name
        );

        // 6. Codegen for macOS arm64
        let macos_arm64 = codegen::generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
        assert!(
            macos_arm64.contains(".globl _main"),
            "Missing _main in macOS ARM64 for '{}'",
            example_name
        );

        // 7. Codegen for macOS x64
        let macos_x64 = codegen::generate(&ast, Architecture::X64, OperatingSystem::MacOS);
        assert!(
            macos_x64.contains(".globl _main"),
            "Missing _main in macOS x64 for '{}'",
            example_name
        );
    }
}

#[test]
fn test_all_examples_execute_with_gcc() {
    if std::process::Command::new("gcc")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping GCC execution: GCC not found in PATH.");
        return;
    }

    let mut examples: Vec<String> = fs::read_dir("examples")
        .expect("Failed to read examples directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("alya") {
                path.file_name()?.to_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    examples.sort();

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

    for (idx, example_name) in examples.iter().enumerate() {
        let path = format!("examples/{}", example_name);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read example file '{}': {}", path, e));

        let mut lexer = Lexer::new(&source);
        let tokens = lexer
            .tokenize()
            .unwrap_or_else(|e| panic!("Lexer failed for '{}': {}", example_name, e));
        let mut parser = Parser::new(tokens);
        let mut ast = parser
            .parse()
            .unwrap_or_else(|e| panic!("Parser failed for '{}': {}", example_name, e));

        let base_dir = std::path::Path::new(&path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        alya::parser::resolve_imports(&mut ast, base_dir)
            .unwrap_or_else(|e| panic!("Import resolution failed for '{}': {}", example_name, e));

        let asm_code = codegen::generate(&ast, arch, os);
        let pid = std::process::id();
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_asm = format!("temp_ex_test_{}_{}_{}.s", pid, idx, time);
        let temp_exe = if cfg!(target_os = "windows") {
            format!("temp_ex_test_{}_{}_{}.exe", pid, idx, time)
        } else {
            format!("temp_ex_test_{}_{}_{}", pid, idx, time)
        };

        fs::write(&temp_asm, &asm_code).expect("Failed to write asm");

        let mut gcc = std::process::Command::new("gcc");
        gcc.arg(&temp_asm).arg("-o").arg(&temp_exe);
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
        gcc.arg("-L.");
        for lib in codegen::collect_extern_libraries(&ast) {
            gcc.arg(format!("-l{}", lib));
        }
        let gcc_status = gcc.status().expect("Failed to run gcc");
        let _ = fs::remove_file(&temp_asm);
        assert!(
            gcc_status.success(),
            "GCC failed to compile '{}'",
            example_name
        );

        let run_cmd = if cfg!(target_os = "windows") {
            format!(".\\{}", temp_exe)
        } else {
            format!("./{}", temp_exe)
        };

        let _ = fs::remove_file("target/demo_file.txt");
        let _ = fs::remove_file("demo_file.txt");

        let mut child = std::process::Command::new(&run_cmd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn compiled example");

        // Pipe mock input for interactive examples like user_input.alya
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(b"TestUser\nAlya\n");
        }

        let output = child
            .wait_with_output()
            .expect("Failed to wait on example execution");
        let _ = fs::remove_file(&temp_exe);
        let _ = fs::remove_file("mini_output.s");

        let actual_stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        let actual_stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
        assert!(
            output.status.success(),
            "Example '{}' failed during execution with status: {:?}\nStdout: {}\nStderr: {}",
            example_name,
            output.status,
            actual_stdout,
            actual_stderr
        );

        let actual_stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        if example_name == "bench_demo.alya" {
            assert!(
                actual_stdout.contains("=== Benchmark Suite: Alya Builtin Micro-Benchmarks ==="),
                "bench_demo missing suite header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("// * Summary *"),
                "bench_demo missing summary section:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Finished 3 benchmark(s) in"),
                "bench_demo missing summary:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "rand_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/rand Demo ==="),
                "rand_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("std/rand demo completed successfully."),
                "rand_demo missing completion:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "cli_demo.alya" {
            assert!(
                actual_stdout.contains("CLI demo completed successfully."),
                "cli_demo missing completion marker:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Lightweight Flag & Option Detection"),
                "cli_demo missing flag detection header:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "csv_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/csv Demo ==="),
                "csv_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Alice (Engineer)"),
                "csv_demo missing record parsing:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("TSV rows: 3"),
                "csv_demo missing TSV:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Serialized Books Records:"),
                "csv_demo missing serialization:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "url_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/url Demo ==="),
                "url_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Scheme   : https"),
                "url_demo missing scheme:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Origin          : https://api.example.com:8443"),
                "url_demo missing origin:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Rebuilt matches raw: 1"),
                "url_demo missing roundtrip:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "log_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/color & std/log Demo ==="),
                "log_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("std/log demo completed successfully."),
                "log_demo missing completion marker:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "glob_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/glob Demo ==="),
                "glob_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("std/glob demo completed successfully."),
                "glob_demo missing completion marker:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "console_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya std/console Demo ==="),
                "console_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("std/console demo completed successfully."),
                "console_demo missing completion marker:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "net_demo.alya" {
            assert!(
                actual_stdout.contains("=== Alya Network & Memory Management Demo ==="),
                "net_demo missing header:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("HTTP Status Code: 200"),
                "net_demo missing status code:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("HTTP Status Text: OK"),
                "net_demo missing status text:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Cloned String: Alya permanent heap string test"),
                "net_demo missing cloned string:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("=== Network & Memory Demo Finished ==="),
                "net_demo missing completion marker:\n{}",
                actual_stdout
            );
            continue;
        }

        if example_name == "concurrency.alya" {
            assert!(
                actual_stdout.contains("Sum: 19, Product: 60"),
                "concurrency missing stats:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Worker 1 total (1..100): 5050"),
                "concurrency missing worker 1:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Worker 2 total (1..50): 1275"),
                "concurrency missing worker 2:\n{}",
                actual_stdout
            );
            assert!(
                actual_stdout.contains("Concurrency test completed successfully."),
                "concurrency missing completion marker:\n{}",
                actual_stdout
            );
            continue;
        }

        let expected = get_expected_output(example_name).unwrap_or_else(|| {
            panic!(
                "Missing expected output definition for example '{}'!",
                example_name
            )
        });

        assert_eq!(
            actual_stdout, expected,
            "Example '{}' output did not match expected output!\nActual:\n{}\nExpected:\n{}",
            example_name, actual_stdout, expected
        );
    }
}
