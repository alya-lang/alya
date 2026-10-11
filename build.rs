fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/brand/icons/alya-dark.ico");
        res.set("FileDescription", "Alya Compiler");
        res.set("ProductName", "Alya");
        res.set("OriginalFilename", "alya.exe");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
        }
    }

    // Build-commit provenance for `alya --version` (asset forensics):
    // records the source revision the binary was built from so a
    // release asset's contents are checkable without rebuilding.
    // Missing git metadata (source tarballs) records "unknown" and
    // the version string omits the suffix.
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=ALYA_BUILD_COMMIT={}", commit);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
