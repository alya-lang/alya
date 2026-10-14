//! Bindgen end-to-end (alya-lang/alya#196): generate bindings from a
//! libc-subset header, compile a program against them, run it.
//! Uses only system libc (always present), so no package checkout.

mod common;

use alya::codegen::{self, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;
use common::{execution_skip_reason, harness_gcc};
use std::fs;

static BINDGEN_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
    } else {
        Architecture::X64
    }
}

#[test]
fn test_bindgen_libc_roundtrip() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP bindgen e2e: {}", reason);
        return;
    }
    if std::process::Command::new(harness_gcc())
        .arg("--version")
        .output()
        .is_err()
    {
        println!("SKIP bindgen e2e: no C toolchain in PATH");
        return;
    }
    // Minimal libc subset: exact-width types only, so every platform maps.
    let header = "size_t strlen(const char *s);\nint strcmp(const char *a, const char *b);\n";
    let generated = alya::tools::bindgen::generate(
        header,
        &alya::tools::bindgen::BindgenOptions { from_lib: None },
    );
    assert!(
        generated.contains("function strlen(s: str) -> usize"),
        "got:\n{}",
        generated
    );
    assert!(
        generated.contains("function strcmp(a: str, b: str) -> i32"),
        "got:\n{}",
        generated
    );

    let id = BINDGEN_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("alya-bindgen-{}-{}", std::process::id(), id));
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("ffi_gen.alya"), &generated).expect("write ffi");
    let main_src = "import \"./ffi_gen.alya\"\n\
        assert strlen(\"hello\") == 5\n\
        assert strcmp(\"a\", \"b\") != 0\n\
        say \"bindgen libc: OK\"\n";
    fs::write(dir.join("main.alya"), main_src).expect("write main");

    let mut lexer = Lexer::new(main_src);
    let tokens = lexer.tokenize().expect("lex");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("parse");
    alya::parser::resolve_imports(&mut ast, &dir, &alya::parser::CfgContext::host())
        .expect("resolve");
    let asm_code = codegen::generate(&ast, host_arch(), host_os());
    let asm_path = dir.join("bindgen.s");
    let exe_path = if cfg!(target_os = "windows") {
        dir.join("bindgen.exe")
    } else {
        dir.join("bindgen")
    };
    fs::write(&asm_path, &asm_code).expect("write asm");
    let mut gcc = std::process::Command::new(harness_gcc());
    gcc.arg(&asm_path).arg("-o").arg(&exe_path);
    if matches!(host_os(), OperatingSystem::Linux) {
        gcc.arg("-no-pie");
        gcc.arg("-lm");
    }
    if matches!(host_os(), OperatingSystem::Windows) {
        gcc.arg("-lws2_32");
    }
    let gcc_out = gcc.output().expect("gcc spawn");
    assert!(
        gcc_out.status.success(),
        "gcc failed:\n{}",
        String::from_utf8_lossy(&gcc_out.stderr)
    );
    let prog_out = std::process::Command::new(&exe_path)
        .output()
        .expect("exec");
    let code = prog_out.status.code().unwrap_or(-1);
    let output = String::from_utf8_lossy(&prog_out.stdout).replace("\r\n", "\n");
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(code, 0, "exit {}\n{}", code, output);
    assert!(output.contains("bindgen libc: OK"), "got:\n{}", output);
}
