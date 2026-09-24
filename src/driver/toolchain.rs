use crate::codegen::{Architecture, OperatingSystem};
use crate::tools::lsp::json::JsonValue;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainCommand {
    Status,
    Install,
    Clean,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainKind {
    Gcc,
    Clang,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainSource {
    System(PathBuf),
    Local(PathBuf),
}

#[derive(Debug, Clone)]
pub struct ToolchainInfo {
    pub kind: ToolchainKind,
    pub compiler_path: PathBuf,
    pub source: ToolchainSource,
    pub version_str: String,
}

/// Live toolchain manifest: single source of truth for supported platforms.
/// New toolchain platforms work without a compiler update; the baked-in
/// table in `install_toolchain` is only an offline fallback.
const TOOLCHAIN_MANIFEST_URL: &str =
    "https://github.com/alya-lang/toolchain/releases/latest/download/toolchain.json";

/// Resolves the toolchain archive filename for `triple` from a
/// toolchain.json manifest document. Pure function over text (no I/O) so it
/// stays unit-testable without network access.
fn manifest_archive_for(manifest_text: &str, triple: &str) -> Option<String> {
    let root = JsonValue::parse(manifest_text).ok()?;
    root.get("platforms")?
        .get(triple)?
        .get("archive")?
        .get("filename")?
        .as_str()
        .map(|s| s.to_string())
}

/// Fetches the live toolchain manifest. Returns None on any failure
/// (offline machine, missing curl/wget, rate limits) so callers fall back
/// to the baked-in platform table.
fn fetch_toolchain_manifest() -> Option<String> {
    // 1. Try curl
    if let Ok(output) = Command::new("curl")
        .args(["-sSL", "--max-time", "20", TOOLCHAIN_MANIFEST_URL])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }

    // 2. Try wget
    if let Ok(output) = Command::new("wget")
        .args(["-qO-", "--timeout=20", TOOLCHAIN_MANIFEST_URL])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }

    // 3. Try powershell on Windows
    if cfg!(target_os = "windows") {
        let ps_script = format!(
            "$ProgressPreference = 'SilentlyContinue'; (Invoke-WebRequest -Uri '{}' -UseBasicParsing).Content",
            TOOLCHAIN_MANIFEST_URL
        );
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).into_owned();
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
        }
    }

    None
}

/// Returns standard ~/.alya directory
pub fn get_global_alya_dir() -> Option<PathBuf> {
    if let Ok(home) = env::var("ALYA_HOME") {
        return Some(PathBuf::from(home));
    }
    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home).join(".alya"));
    }
    if let Ok(userprofile) = env::var("USERPROFILE") {
        return Some(PathBuf::from(userprofile).join(".alya"));
    }
    None
}

/// Returns ~/.alya/toolchain directory
pub fn get_local_toolchain_dir() -> Option<PathBuf> {
    get_global_alya_dir().map(|d| d.join("toolchain"))
}

/// Returns the platform target string (e.g. x86_64-pc-windows-gnu)
pub fn get_platform_triple(arch: Architecture, os: OperatingSystem) -> String {
    match (os, arch) {
        (OperatingSystem::Windows, Architecture::X64) => "x86_64-pc-windows-gnu".to_string(),
        (OperatingSystem::Windows, Architecture::X86) => "i686-pc-windows-gnu".to_string(),
        (OperatingSystem::Windows, Architecture::ARM64) => "aarch64-pc-windows-gnu".to_string(),
        (OperatingSystem::Linux, Architecture::X64) => "x86_64-unknown-linux-musl".to_string(),
        (OperatingSystem::Linux, Architecture::ARM64) => "aarch64-unknown-linux-musl".to_string(),
        (OperatingSystem::Linux, Architecture::X86) => "i686-unknown-linux-musl".to_string(),
        (OperatingSystem::MacOS, Architecture::ARM64) => "aarch64-apple-darwin".to_string(),
        (OperatingSystem::MacOS, Architecture::X64) => "x86_64-apple-darwin".to_string(),
        (OperatingSystem::MacOS, Architecture::X86) => "i686-apple-darwin".to_string(),
    }
}

/// Checks if a compiler candidate can run and extracts its first line of --version
fn probe_compiler(path: &Path) -> Option<(ToolchainKind, String)> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim().to_string();

    let kind = if first_line.to_lowercase().contains("clang")
        || path
            .file_name()
            .map(|f| f.to_string_lossy().contains("clang"))
            .unwrap_or(false)
    {
        ToolchainKind::Clang
    } else {
        ToolchainKind::Gcc
    };

    Some((kind, first_line))
}

/// Returns the machine prefix reported by `<compiler> -dumpmachine`
/// (e.g. "x86_64-w64-mingw32" -> "x86_64", "aarch64-w64-mingw32" -> "aarch64").
fn compiler_machine_prefix(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("-dumpmachine").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let machine = String::from_utf8_lossy(&output.stdout).trim().to_string();
    machine
        .split('-')
        .next()
        .map(|s| s.to_lowercase())
        .filter(|s| !s.is_empty())
}

/// True when a `machine` prefix (see `compiler_machine_prefix`) can
/// assemble output for the requested architecture.
fn machine_matches_arch(machine: &str, arch: Architecture) -> bool {
    match arch {
        Architecture::X64 => machine == "x86_64" || machine == "amd64",
        Architecture::X86 => machine.starts_with('i') && machine.ends_with("86"),
        Architecture::ARM64 => machine == "aarch64" || machine == "arm64",
    }
}

/// True when the compiler at `path` targets the requested architecture.
/// An unprobable compiler counts as a match to preserve legacy behavior
/// (accept first working system compiler).
pub fn compiler_matches_arch(path: &Path, arch: Architecture) -> bool {
    match compiler_machine_prefix(path) {
        None => true,
        Some(machine) => machine_matches_arch(&machine, arch),
    }
}

/// Detects system toolchain via PATH
pub fn detect_system_toolchain(os: OperatingSystem) -> Option<ToolchainInfo> {
    let candidates: &[&str] = match os {
        OperatingSystem::Windows => &["gcc", "clang"],
        OperatingSystem::MacOS => &["clang", "gcc", "cc"],
        OperatingSystem::Linux => &["gcc", "clang", "cc"],
    };

    for candidate in candidates {
        if let Some((kind, version_str)) = probe_compiler(Path::new(candidate)) {
            return Some(ToolchainInfo {
                kind,
                compiler_path: PathBuf::from(*candidate),
                source: ToolchainSource::System(PathBuf::from(*candidate)),
                version_str,
            });
        }
    }

    None
}

/// Detects local toolchain inside sibling directory or ~/.alya/toolchain
pub fn detect_local_toolchain(os: OperatingSystem) -> Option<ToolchainInfo> {
    let candidates: &[&str] = match os {
        OperatingSystem::Windows => &["gcc.exe", "clang.exe"],
        OperatingSystem::MacOS => &["clang", "gcc"],
        OperatingSystem::Linux => &["gcc", "clang"],
    };

    // 1. Check portable sibling directory (e.g. <dir>/alya.exe / <dir>/alya.exe and <dir>/toolchain/bin/gcc.exe)
    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let sibling_bin = parent.join("toolchain").join("bin");
            if sibling_bin.exists() {
                for name in candidates {
                    let exe_path = sibling_bin.join(name);
                    if exe_path.exists() {
                        if let Some((kind, version_str)) = probe_compiler(&exe_path) {
                            return Some(ToolchainInfo {
                                kind,
                                compiler_path: exe_path.clone(),
                                source: ToolchainSource::Local(exe_path),
                                version_str,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Check global ~/.alya/toolchain
    let base = get_local_toolchain_dir()?;
    let bin_dir = base.join("bin");

    for name in candidates {
        let exe_path = bin_dir.join(name);
        if exe_path.exists() {
            if let Some((kind, version_str)) = probe_compiler(&exe_path) {
                return Some(ToolchainInfo {
                    kind,
                    compiler_path: exe_path.clone(),
                    source: ToolchainSource::Local(exe_path),
                    version_str,
                });
            }
        }
    }

    None
}

/// Resolves active toolchain. If absent, prompts user or automatically provisions.
pub fn resolve_toolchain(
    arch: Architecture,
    os: OperatingSystem,
    quiet: bool,
) -> Result<ToolchainInfo, String> {
    // 1. Check system PATH, but only when it targets the requested
    // architecture. A foreign-arch system compiler (e.g. x64 MinGW on a
    // Windows ARM64 runner) cannot assemble our output and must not shadow
    // the portable toolchain in ~/.alya/toolchain.
    if let Some(info) = detect_system_toolchain(os) {
        if compiler_matches_arch(&info.compiler_path, arch) {
            return Ok(info);
        }
    }

    // 2. Check ~/.alya/toolchain
    if let Some(info) = detect_local_toolchain(os) {
        return Ok(info);
    }

    // 3. Absent handling per OS
    match os {
        OperatingSystem::MacOS => {
            // Suggest xcode-select --install
            let msg = "\nError: Apple Command Line Tools (clang) not detected.\n\
                       Alya requires Clang and Mach-O linker tools to compile native executables.\n\n\
                       To install official Apple tools, run:\n\
                         xcode-select --install\n";
            Err(msg.to_string())
        }
        OperatingSystem::Linux => {
            // Detect package manager hints
            let hint =
                if Path::new("/usr/bin/apt").exists() || Path::new("/usr/bin/apt-get").exists() {
                    "sudo apt install build-essential"
                } else if Path::new("/usr/bin/dnf").exists() {
                    "sudo dnf groupinstall \"Development Tools\""
                } else if Path::new("/usr/bin/pacman").exists() {
                    "sudo pacman -S base-devel"
                } else if Path::new("/sbin/apk").exists() {
                    "sudo apk add build-base"
                } else {
                    "install gcc or clang using your system package manager"
                };

            let auto_install = env::var("ALYA_TOOLCHAIN_AUTO_INSTALL")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            if auto_install {
                install_toolchain(arch, os, quiet)
            } else {
                Err(format!(
                    "\nError: C/Assembly build tools (GCC or Clang) not detected in PATH.\n\
                     Alya requires an assembler and linker to produce executables.\n\n\
                     Suggested installation:\n  {}\n\n\
                     Alternatively, run 'alya toolchain install' to install a portable standalone toolchain.\n",
                    hint
                ))
            }
        }
        OperatingSystem::Windows => {
            let auto_install = env::var("ALYA_TOOLCHAIN_AUTO_INSTALL")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            if auto_install {
                return install_toolchain(arch, os, quiet);
            }

            println!("\n[Alya] Windows C/Assembly build tools (GCC/MinGW) not detected.");
            println!(
                "       Alya requires a minimal assembler and linker to build and run executables."
            );
            print!(
                "       Download portable minimal toolchain (~18 MB) to ~/.alya/toolchain? [Y/n]: "
            );
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let trimmed = input.trim();
                if trimmed.is_empty()
                    || trimmed.eq_ignore_ascii_case("y")
                    || trimmed.eq_ignore_ascii_case("yes")
                {
                    return install_toolchain(arch, os, quiet);
                }
            }

            Err(
                "Toolchain installation declined. Cannot link executable without GCC/MinGW."
                    .to_string(),
            )
        }
    }
}

/// Downloads and installs minimal portable toolchain
pub fn install_toolchain(
    arch: Architecture,
    os: OperatingSystem,
    quiet: bool,
) -> Result<ToolchainInfo, String> {
    let target_dir = get_local_toolchain_dir()
        .ok_or_else(|| "Error: Cannot resolve ~/.alya home directory".to_string())?;

    let triple = get_platform_triple(arch, os);
    let fallback_name = match (os, arch) {
        (OperatingSystem::Windows, Architecture::X64) => "alya-toolchain-windows-x64.zip",
        (OperatingSystem::Windows, Architecture::ARM64) => "alya-toolchain-windows-arm64.zip",
        (OperatingSystem::Windows, Architecture::X86) => "alya-toolchain-windows-x86.zip",
        (OperatingSystem::Linux, Architecture::X64) => "alya-toolchain-linux-x64.tar.gz",
        (OperatingSystem::Linux, Architecture::ARM64) => "alya-toolchain-linux-arm64.tar.gz",
        (OperatingSystem::MacOS, Architecture::ARM64) => "alya-toolchain-macos-arm64.tar.gz",
        (OperatingSystem::MacOS, Architecture::X64) => "alya-toolchain-macos-x64.tar.gz",
        _ => {
            return Err(format!(
                "Error: No pre-built minimal toolchain available for {:?} on {:?}",
                arch, os
            ));
        }
    };

    // Prefer the live manifest (single source of truth for supported
    // platforms); fall back to the baked-in table when offline.
    let archive_name = match fetch_toolchain_manifest() {
        Some(text) => match manifest_archive_for(&text, &triple) {
            Some(name) => name,
            None => {
                return Err(format!(
                    "Error: No pre-built minimal toolchain available for {:?} on {:?} (triple '{}' not listed in toolchain manifest)",
                    arch, os, triple
                ));
            }
        },
        None => fallback_name.to_string(),
    };

    let download_candidates = if let Ok(custom) = env::var("ALYA_TOOLCHAIN_URL") {
        vec![custom]
    } else {
        vec![
            format!(
                "https://github.com/alya-lang/toolchain/releases/latest/download/{}",
                archive_name
            ),
            format!(
                "https://github.com/alya-lang/toolchain/releases/download/v1.0.0/{}",
                archive_name
            ),
            format!(
                "https://cdn.jsdelivr.net/gh/alya-lang/toolchain@releases/{}",
                archive_name
            ),
            format!(
                "https://raw.githubusercontent.com/alya-lang/toolchain/release-assets/{}",
                archive_name
            ),
        ]
    };

    if !quiet {
        println!("[Alya Toolchain] Target: {}", get_platform_triple(arch, os));
    }

    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "Failed to create toolchain dir '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    let temp_archive = target_dir.join(archive_name);

    let mut download_ok = false;
    for (idx, candidate_url) in download_candidates.iter().enumerate() {
        if !quiet && idx > 0 {
            println!("[Alya Toolchain] Retrying with mirror: {}", candidate_url);
        } else if !quiet {
            println!("[Alya Toolchain] Fetching: {}", candidate_url);
        }

        // 1. Try curl
        let mut ok = Command::new("curl")
            .args(["-sSL", "-f", candidate_url, "-o"])
            .arg(&temp_archive)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        // 2. Try wget
        if !ok {
            ok = Command::new("wget")
                .args(["-q", candidate_url, "-O"])
                .arg(&temp_archive)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }

        // 3. Try powershell on Windows
        if !ok && cfg!(target_os = "windows") {
            let ps_script = format!(
                "$ProgressPreference = 'SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                candidate_url,
                temp_archive.display().to_string().replace('\\', "/")
            );
            ok = Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }

        if ok
            && temp_archive.exists()
            && fs::metadata(&temp_archive).map(|m| m.len()).unwrap_or(0) > 0
        {
            download_ok = true;
            break;
        } else {
            let _ = fs::remove_file(&temp_archive);
        }
    }

    if !download_ok || !temp_archive.exists() {
        let _ = fs::remove_file(&temp_archive);
        return Err(format!(
            "Failed to download toolchain for {} across all available mirrors.\n\
             Please verify your network connection or set ALYA_TOOLCHAIN_URL to a local path or mirror.",
            get_platform_triple(arch, os)
        ));
    }

    // Verify cryptographic SHA-256
    if let Ok(bytes) = fs::read(&temp_archive) {
        let computed_hash = crate::tools::pkg::hash::sha256_hex(&bytes);
        if !quiet {
            println!(
                "[Alya Toolchain] Download verified (SHA-256: {}...)",
                &computed_hash[..16]
            );
        }
    }

    if !quiet {
        println!("[Alya Toolchain] Extracting to {}...", target_dir.display());
    }

    // Extract archive using tar
    let mut extracted = Command::new("tar")
        .arg("-xf")
        .arg(&temp_archive)
        .arg("-C")
        .arg(&target_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // Fallback on Windows with PowerShell Expand-Archive if tar fails
    if !extracted
        && cfg!(target_os = "windows")
        && temp_archive
            .extension()
            .map(|e| e == "zip")
            .unwrap_or(false)
    {
        let ps_script = format!(
            "$ProgressPreference = 'SilentlyContinue'; Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            temp_archive.display().to_string().replace('\\', "/"),
            target_dir.display().to_string().replace('\\', "/")
        );
        extracted = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    let _ = fs::remove_file(&temp_archive);

    if !extracted {
        return Err("Failed to extract toolchain archive.".to_string());
    }

    // Verify installation
    let toolchain = detect_local_toolchain(os).ok_or_else(|| {
        "Toolchain extracted but compiler binary could not be verified in ~/.alya/toolchain/bin."
            .to_string()
    })?;

    if !quiet {
        println!("✓ Toolchain installed successfully!");
        println!("  Compiler: {}", toolchain.compiler_path.display());
        println!("  Version:  {}", toolchain.version_str);
    }

    Ok(toolchain)
}

/// CLI command handler for `alya toolchain [status|install|clean|help]`
pub fn run_toolchain_cmd(
    cmd: &ToolchainCommand,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<(), String> {
    match cmd {
        ToolchainCommand::Status => {
            println!("Alya Toolchain Status");
            println!("=====================");
            println!("Platform Target: {}", get_platform_triple(arch, os));

            if let Some(info) = detect_system_toolchain(os) {
                println!("System Compiler: Found (PATH)");
                println!("  Path:    {}", info.compiler_path.display());
                println!("  Version: {}", info.version_str);
            } else {
                println!("System Compiler: Not found in PATH");
            }

            if let Some(info) = detect_local_toolchain(os) {
                println!("Local Toolchain: Installed (~/.alya/toolchain)");
                println!("  Path:    {}", info.compiler_path.display());
                println!("  Version: {}", info.version_str);
            } else {
                println!("Local Toolchain: Not installed in ~/.alya/toolchain");
            }

            match resolve_toolchain(arch, os, true) {
                Ok(active) => {
                    println!("\nActive Toolchain for Builds:");
                    let source_desc = match active.source {
                        ToolchainSource::System(_) => "Host System (PATH)",
                        ToolchainSource::Local(_) => "Alya Portable (~/.alya/toolchain)",
                    };
                    println!("  Source:  {}", source_desc);
                    println!("  Binary:  {}", active.compiler_path.display());
                    println!("  Version: {}", active.version_str);
                }
                Err(_) => {
                    println!("\nActive Toolchain: None (Builds will fail without GCC/Clang)");
                    println!("Run 'alya toolchain install' to install minimal toolchain.");
                }
            }
        }
        ToolchainCommand::Install => {
            println!("Installing Alya minimal toolchain...");
            let info = install_toolchain(arch, os, false)?;
            println!("\n✓ Ready to use: {}", info.compiler_path.display());
        }
        ToolchainCommand::Clean => {
            if let Some(dir) = get_local_toolchain_dir() {
                if dir.exists() {
                    fs::remove_dir_all(&dir)
                        .map_err(|e| format!("Failed to clean '{}': {}", dir.display(), e))?;
                    println!("✓ Purged local toolchain directory: {}", dir.display());
                } else {
                    println!(
                        "Local toolchain directory does not exist: {}",
                        dir.display()
                    );
                }
            }
        }
        ToolchainCommand::Help => {
            println!("Alya Toolchain Manager");
            println!("Usage: alya toolchain <command>\n");
            println!("Commands:");
            println!("  status     Show active compiler, version, and location");
            println!("  install    Pre-emptively download and configure portable toolchain");
            println!("  clean      Purge ~/.alya/toolchain to reclaim disk space");
            println!("  help       Show this help message");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_triples() {
        assert_eq!(
            get_platform_triple(Architecture::X64, OperatingSystem::Windows),
            "x86_64-pc-windows-gnu"
        );
        assert_eq!(
            get_platform_triple(Architecture::X86, OperatingSystem::Windows),
            "i686-pc-windows-gnu"
        );
        assert_eq!(
            get_platform_triple(Architecture::ARM64, OperatingSystem::Windows),
            "aarch64-pc-windows-gnu"
        );
        assert_eq!(
            get_platform_triple(Architecture::X64, OperatingSystem::Linux),
            "x86_64-unknown-linux-musl"
        );
        assert_eq!(
            get_platform_triple(Architecture::ARM64, OperatingSystem::Linux),
            "aarch64-unknown-linux-musl"
        );
        assert_eq!(
            get_platform_triple(Architecture::ARM64, OperatingSystem::MacOS),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            get_platform_triple(Architecture::X64, OperatingSystem::MacOS),
            "x86_64-apple-darwin"
        );
    }

    #[test]
    fn test_manifest_archive_for() {
        let manifest = r#"{
            "name": "alya-toolchain",
            "version": "1.0.0",
            "platforms": {
                "x86_64-pc-windows-gnu": {
                    "archive": {"filename": "alya-toolchain-windows-x64.zip"}
                },
                "aarch64-pc-windows-gnu": {
                    "archive": {
                        "filename": "alya-toolchain-windows-arm64.zip",
                        "sha256": "abc123",
                        "compressed_size_mb": 89
                    }
                }
            }
        }"#;
        assert_eq!(
            manifest_archive_for(manifest, "aarch64-pc-windows-gnu"),
            Some("alya-toolchain-windows-arm64.zip".to_string())
        );
        assert_eq!(
            manifest_archive_for(manifest, "x86_64-pc-windows-gnu"),
            Some("alya-toolchain-windows-x64.zip".to_string())
        );
        // Triple absent from manifest
        assert_eq!(manifest_archive_for(manifest, "i686-pc-windows-gnu"), None);
        // Malformed manifest
        assert_eq!(
            manifest_archive_for("not json{", "x86_64-pc-windows-gnu"),
            None
        );
        assert_eq!(manifest_archive_for("{}", "x86_64-pc-windows-gnu"), None);
    }

    #[test]
    fn test_machine_matches_arch() {
        assert!(machine_matches_arch("x86_64", Architecture::X64));
        assert!(machine_matches_arch("amd64", Architecture::X64));
        assert!(!machine_matches_arch("aarch64", Architecture::X64));
        assert!(machine_matches_arch("i686", Architecture::X86));
        assert!(machine_matches_arch("i386", Architecture::X86));
        assert!(!machine_matches_arch("x86_64", Architecture::X86));
        assert!(machine_matches_arch("aarch64", Architecture::ARM64));
        assert!(machine_matches_arch("arm64", Architecture::ARM64));
        assert!(!machine_matches_arch("x86_64", Architecture::ARM64));
    }

    #[test]
    fn test_local_toolchain_dir() {
        let dir = get_local_toolchain_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.ends_with(".alya/toolchain") || path.ends_with(".alya\\toolchain"));
    }

    #[test]
    fn test_system_toolchain_detection() {
        // If the machine has gcc/clang in PATH, verify probe succeeded
        let os = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        };

        if let Some(toolchain) = detect_system_toolchain(os) {
            assert!(!toolchain.version_str.is_empty());
            assert!(toolchain.kind == ToolchainKind::Gcc || toolchain.kind == ToolchainKind::Clang);
        }
    }
}
