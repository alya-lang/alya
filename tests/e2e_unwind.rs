//! Windows-x64 SEH unwinder coverage (alya-lang/alya#109).
//!
//! Runs the `fixtures/unwind` package (Alya + C probe) with the
//! freshly-built `alya` binary: asserts `.pdata` describes Alya frames
//! and that `longjmp` (RtlUnwindEx) survives unwinding THROUGH an
//! exported Alya frame. Windows-x64 only — `.seh_*` is PE/COFF;
//! other targets skip (their unwind story is unchanged).

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
#[test]
fn test_e2e_seh_unwind_through_alya_frames() {
    let alya = env!("CARGO_BIN_EXE_alya");
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests").join("fixtures").join("unwind");
    // gcc must exist (execution leg, like the e2e harness gate).
    if std::process::Command::new("gcc")
        .arg("--version")
        .output()
        .is_err()
    {
        println!("SKIP unwind probe: no gcc in PATH");
        return;
    }
    let out = std::process::Command::new(alya)
        .arg("run")
        .arg("main.alya")
        .current_dir(&fixture)
        .output()
        .expect("alya run failed to spawn");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    assert_eq!(
        code, 0,
        "probe crashed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("pdata=1\n"),
        "Alya frames missing from .pdata.\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("unwind=42\n"),
        "unwind through Alya frame failed.\nstdout:\n{stdout}"
    );
}

#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
#[test]
fn test_e2e_seh_unwind_through_alya_frames() {
    println!("SKIP unwind probe: Windows-x64 only (.pdata/.xdata)");
}
