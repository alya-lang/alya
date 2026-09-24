mod common;
use alya::codegen::{self, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;
use common::execution_skip_reason;
use std::fs;
use std::path::{Path, PathBuf};

fn get_integration_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let embedded = manifest_dir.join("spec").join("integration");
    if embedded.exists() {
        embedded
    } else {
        manifest_dir
            .parent()
            .unwrap()
            .join("spec")
            .join("integration")
    }
}

// Entry points only. Helpers (e.g. 02_facade_types.alya) are imported by
// entries and must NOT be executed standalone.
fn entry_files() -> Vec<String> {
    vec![
        "01_toml_mini.alya".to_string(),
        "02_facade_main.alya".to_string(),
        "03_ffi_struct.alya".to_string(),
    ]
}

fn host_os() -> OperatingSystem {
    if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    }
}

fn host_arch() -> Architecture {
    if cfg!(target_arch = "aarch64") {
        Architecture::ARM64
    } else if cfg!(target_arch = "x86") {
        Architecture::X86
    } else {
        Architecture::X64
    }
}

fn run_entry_with_base_dir(source: &str, base_dir: &Path) -> Option<(i32, String)> {
    if std::process::Command::new("gcc")
        .arg("--version")
        .output()
        .is_err()
    {
        return None;
    }
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parser error");
    let test_entries = {
        use alya::tools::test_runner::{discover_suite_entry_points, SuiteKind};
        discover_suite_entry_points(&ast, SuiteKind::Test)
    };
    alya::parser::resolve_imports(&mut ast, base_dir).expect("Import resolution failed");
    {
        use alya::tools::test_runner::synthesize_test_calls;
        synthesize_test_calls(&mut ast, &test_entries);
    }

    let os = host_os();
    let arch = host_arch();
    let asm_code = codegen::generate(&ast, arch, os);

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let asm_path = format!("temp_integ_{}_{}.s", std::process::id(), nanos);
    let exe_path = if cfg!(target_os = "windows") {
        format!("temp_integ_{}_{}.exe", std::process::id(), nanos)
    } else {
        format!("temp_integ_{}_{}", std::process::id(), nanos)
    };
    fs::write(&asm_path, &asm_code).expect("Failed to write temp asm");

    let mut gcc = std::process::Command::new(common::harness_gcc());
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
    let prog_out = std::process::Command::new(&run_cmd)
        .output()
        .expect("Program execution failed");
    let _ = fs::remove_file(&exe_path);
    let code = prog_out.status.code().unwrap_or(-1);
    let mut output = String::from_utf8_lossy(&prog_out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&prog_out.stderr).replace("\r\n", "\n");
    output.push_str(&stderr);
    Some((code, output))
}

#[test]
fn test_integration_entries_exist() {
    let dir = get_integration_dir();
    assert!(dir.exists(), "spec/integration must exist at {:?}", dir);
    for name in entry_files() {
        assert!(
            dir.join(&name).exists(),
            "Integration entry missing: {:?}",
            dir.join(&name)
        );
    }
    // Helper must exist but must NOT be listed as an entry.
    assert!(dir.join("02_facade_types.alya").exists());
    assert!(!entry_files().contains(&"02_facade_types.alya".to_string()));
}

#[test]
fn test_integration_lex_parse_resolve() {
    let dir = get_integration_dir();
    for name in entry_files() {
        let source = fs::read_to_string(dir.join(&name)).unwrap();
        let mut lexer = Lexer::new(&source);
        let tokens = lexer
            .tokenize()
            .unwrap_or_else(|e| panic!("Lexer failed for '{}': {}", name, e));
        let mut parser = Parser::new(tokens);
        let mut ast = parser
            .parse()
            .unwrap_or_else(|e| panic!("Parser failed for '{}': {}", name, e));
        alya::parser::resolve_imports(&mut ast, &dir)
            .unwrap_or_else(|e| panic!("Import resolution failed for '{}': {}", name, e));
        println!("  [INTEG PARSE OK] {}", name);
    }
}

#[test]
fn test_integration_codegen_matrix() {
    let dir = get_integration_dir();
    for name in entry_files() {
        let source = fs::read_to_string(dir.join(&name)).unwrap();
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize().expect("Lexer error");
        let mut parser = Parser::new(tokens);
        let mut ast = parser.parse().expect("Parser error");
        alya::parser::resolve_imports(&mut ast, &dir).expect("Import resolution failed");
        for (arch, os) in [
            (Architecture::X64, OperatingSystem::Windows),
            (Architecture::X64, OperatingSystem::Linux),
            (Architecture::X86, OperatingSystem::Linux),
            (Architecture::ARM64, OperatingSystem::Linux),
            (Architecture::ARM64, OperatingSystem::MacOS),
        ] {
            let asm = alya::codegen::generate(&ast, arch, os);
            assert!(!asm.is_empty(), "Empty assembly for '{}'", name);
        }
        println!("  [INTEG CODEGEN OK] {}", name);
    }
}

#[test]
fn test_integration_execution_matrix() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP integration execution: {}", reason);
        return;
    }
    let dir = get_integration_dir();
    for name in entry_files() {
        let source = fs::read_to_string(dir.join(&name)).unwrap();
        if let Some((code, output)) = run_entry_with_base_dir(&source, &dir) {
            assert_eq!(
                code, 0,
                "Entry '{}' exit={}\nOutput:\n{}",
                name, code, output
            );
            println!("  [INTEG EXEC OK] {}", name);
        }
    }
}

#[test]
fn test_integration_01_toml_mini_output() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP 01_toml_mini: {}", reason);
        return;
    }
    let dir = get_integration_dir();
    let source = fs::read_to_string(dir.join("01_toml_mini.alya")).unwrap();
    if let Some((code, output)) = run_entry_with_base_dir(&source, &dir) {
        assert_eq!(code, 0, "Output:\n{}", output);
        assert!(output.contains("integration 01_toml_mini: OK"));
    }
}

#[test]
fn test_integration_02_facade_output() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP 02_facade: {}", reason);
        return;
    }
    let dir = get_integration_dir();
    let source = fs::read_to_string(dir.join("02_facade_main.alya")).unwrap();
    if let Some((code, output)) = run_entry_with_base_dir(&source, &dir) {
        assert_eq!(code, 0, "Output:\n{}", output);
        assert!(output.contains("integration 02_facade: OK"));
        assert!(output.contains("Alice (count: 3)"));
    }
}

#[test]
fn test_integration_03_ffi_struct_output() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP 03_ffi_struct: {}", reason);
        return;
    }
    let dir = get_integration_dir();
    let source = fs::read_to_string(dir.join("03_ffi_struct.alya")).unwrap();
    if let Some((code, output)) = run_entry_with_base_dir(&source, &dir) {
        assert_eq!(code, 0, "Output:\n{}", output);
        assert!(output.contains("integration 03_ffi_struct: OK"));
        assert!(output.contains("abs(-42) via libc: 42"));
    }
}

#[test]
fn test_integration_private_symbol_is_rejected() {
    // pub boundary: importing a private helper must fail at resolve time.
    let dir = get_integration_dir();
    let source = "from \"./02_facade_types.alya\" import sanitize_text_internal\nsay sanitize_text_internal(\"x\", \"y\")";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parser error");
    let err = alya::parser::resolve_imports(&mut ast, &dir).unwrap_err();
    assert!(
        err.contains("private symbol"),
        "Expected private-symbol rejection, got: {}",
        err
    );
}

#[test]
fn test_native_execution_support_or_documented_skip() {
    // Same gate as golden: prove execution with a smoke binary, or print the
    // recorded skip reason. Test name keeps the gap auditable in CI logs.
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP native execution on this platform: {}", reason);
        return;
    }
    let dir = get_integration_dir();
    match run_entry_with_base_dir("say 40\nsay 2\n", &dir) {
        None => println!("SKIP smoke: no C toolchain in PATH"),
        Some((code, out)) => {
            assert_eq!(code, 0, "smoke binary exit code");
            assert_eq!(out, "40\n2\n", "smoke binary output");
        }
    }
}
