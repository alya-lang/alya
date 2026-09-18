use super::discovery::find_manifest_dir;
use super::lock::parse_lockfile;
use super::manifest::parse_manifest;
use super::types::{CachedPackageDetails, PackageLock};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn get_global_cache_dir() -> Option<PathBuf> {
    get_global_alya_dir().map(|d| d.join("cache"))
}

pub fn get_global_c_obj_dir() -> Option<PathBuf> {
    get_global_alya_dir().map(|d| d.join("c_obj"))
}

pub fn dir_size_and_count(path: &Path) -> (u64, usize) {
    let mut total_size = 0u64;
    let mut file_count = 0usize;
    if !path.exists() {
        return (0, 0);
    }
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n == ".git") {
                        continue;
                    }
                    stack.push(p);
                } else if p.is_file() {
                    file_count += 1;
                    if let Ok(meta) = p.metadata() {
                        total_size += meta.len();
                    }
                }
            }
        }
    }
    (total_size, file_count)
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b < KB {
        format!("{} B", bytes)
    } else if b < MB {
        format!("{:.2} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

pub fn inspect_packages_dir(dir: &Path, lock: Option<&PackageLock>) -> Vec<CachedPackageDetails> {
    let mut result = Vec::new();
    if !dir.exists() {
        return result;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                if folder_name.starts_with('.') || folder_name == "c_obj" {
                    continue;
                }
                let mut name = folder_name
                    .split('@')
                    .next()
                    .unwrap_or(&folder_name)
                    .to_string();
                let folder_tag = folder_name
                    .split('@')
                    .nth(1)
                    .and_then(|s| s.split('-').next())
                    .unwrap_or("unknown")
                    .to_string();
                let mut version = folder_tag.clone();
                let mut source = "-".to_string();
                let manifest_path = path.join("alya.toml");

                if manifest_path.exists() {
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        if let Ok(m) = parse_manifest(&content) {
                            name = m.package.name.clone();
                            let ver = m.package.version.trim();
                            if !ver.is_empty() {
                                let v_str = if ver.starts_with('v') || ver.starts_with('V') {
                                    ver.to_string()
                                } else {
                                    format!("v{}", ver)
                                };
                                let is_semver = folder_tag
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_digit())
                                    || (folder_tag.starts_with('v')
                                        && folder_tag
                                            .chars()
                                            .nth(1)
                                            .is_some_and(|c| c.is_ascii_digit()));
                                if !is_semver && folder_tag != "unknown" {
                                    version = format!("{} ({})", folder_tag, v_str);
                                } else {
                                    version = v_str;
                                }
                            }
                        }
                    }
                }

                if let Some(l) = lock {
                    if let Some(lp) = l.packages.iter().find(|p| p.name == name) {
                        if !lp.version.is_empty() {
                            let ver = lp.version.trim();
                            version = if ver.starts_with('v') || ver.starts_with('V') {
                                ver.to_string()
                            } else {
                                format!("v{}", ver)
                            };
                        }
                        source = lp.source.clone();
                    }
                } else {
                    let source_file = path.join(".alya-source");
                    if source_file.exists() {
                        if let Ok(s) = fs::read_to_string(&source_file) {
                            let trimmed = s.trim();
                            if !trimmed.is_empty() {
                                source = trimmed.to_string();
                            }
                        }
                    }
                }

                let (size_bytes, file_count) = dir_size_and_count(&path);
                result.push(CachedPackageDetails {
                    name,
                    version,
                    source,
                    size_bytes,
                    file_count,
                    path,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    result
}

pub fn run_cache() -> Result<(), String> {
    println!("Alya Package Cache & Storage\n");

    let manifest_dir = find_manifest_dir();
    let global_dir = get_global_cache_dir();
    let mut total_packages = 0usize;
    let mut total_bytes = 0u64;

    // 1. Local Project Cache (if inside an Alya project)
    if let Some(ref m_dir) = manifest_dir {
        let packages_dir = m_dir.join(".alya").join("packages");
        let lock_path = if m_dir.join("alya.lock").exists() {
            m_dir.join("alya.lock")
        } else {
            m_dir.join("Alya.lock")
        };
        let lock = if lock_path.exists() {
            fs::read_to_string(&lock_path)
                .ok()
                .and_then(|c| parse_lockfile(&c).ok())
        } else {
            None
        };

        let local_pkgs = inspect_packages_dir(&packages_dir, lock.as_ref());
        let (local_size, local_files) = dir_size_and_count(&packages_dir);
        total_packages += local_pkgs.len();
        total_bytes += local_size;

        println!("[Local Project Cache]");
        println!("  Location:     {}", packages_dir.display());
        println!(
            "  Total Size:   {} ({} files)",
            format_bytes(local_size),
            local_files
        );
        println!("  Packages:     {}", local_pkgs.len());

        if !local_pkgs.is_empty() {
            println!();
            println!(
                "  {:<16} {:<16} {:<36} {:<10} {:<8}",
                "PACKAGE", "VERSION", "SOURCE", "SIZE", "FILES"
            );
            println!("  {}", "-".repeat(90));
            for p in &local_pkgs {
                let src_short = if p.source.len() > 34 {
                    format!("{}...", &p.source[..31])
                } else {
                    p.source.clone()
                };
                println!(
                    "  {:<16} {:<16} {:<36} {:<10} {:<8}",
                    p.name,
                    p.version,
                    src_short,
                    format_bytes(p.size_bytes),
                    p.file_count
                );
            }
        } else {
            println!("  (no packages installed in .alya/packages)");
        }
        println!();
    }

    // 2. Global Cache
    if let Some(ref g_dir) = global_dir {
        let (global_size, global_files) = dir_size_and_count(g_dir);
        let global_pkgs = inspect_packages_dir(g_dir, None);
        total_packages += global_pkgs.len();
        total_bytes += global_size;

        println!("[Global Cache]");
        println!("  Location:     {}", g_dir.display());
        println!(
            "  Total Size:   {} ({} files)",
            format_bytes(global_size),
            global_files
        );
        println!("  Packages:     {}", global_pkgs.len());

        if !global_pkgs.is_empty() {
            println!();
            println!(
                "  {:<16} {:<16} {:<36} {:<10} {:<8}",
                "PACKAGE", "VERSION", "SOURCE", "SIZE", "FILES"
            );
            println!("  {}", "-".repeat(90));
            for p in &global_pkgs {
                let src_short = if p.source.len() > 34 {
                    format!("{}...", &p.source[..31])
                } else {
                    p.source.clone()
                };
                println!(
                    "  {:<16} {:<16} {:<36} {:<10} {:<8}",
                    p.name,
                    p.version,
                    src_short,
                    format_bytes(p.size_bytes),
                    p.file_count
                );
            }
        } else {
            println!("  (global cache is empty)");
        }
        println!();
    }

    println!(
        "Summary: {} stored in {} package(s).",
        format_bytes(total_bytes),
        total_packages
    );
    if total_bytes > 0 {
        println!(
            "Tip: Run 'alya pkg clean' or 'alya pkg cache clean' to remove cached packages."
        );
    }

    Ok(())
}

pub fn run_clean(all: bool) -> Result<(), String> {
    let mut cleaned_bytes = 0u64;
    let mut cleaned_items = 0usize;
    let manifest_dir = find_manifest_dir();

    // 1. Clean local project packages
    if let Some(ref m_dir) = manifest_dir {
        let packages_dir = m_dir.join(".alya").join("packages");
        if packages_dir.exists() {
            let (size, count) = dir_size_and_count(&packages_dir);
            if fs::remove_dir_all(&packages_dir).is_ok() {
                cleaned_bytes += size;
                cleaned_items += count;
                println!(
                    "✓ Cleaned local package cache: {} ({} freed)",
                    packages_dir.display(),
                    format_bytes(size)
                );
            }
        }
    }

    // 2. Clean global cache (if requested via --all or if outside any project)
    if all || manifest_dir.is_none() {
        if let Some(g_dir) = get_global_cache_dir() {
            if g_dir.exists() {
                let (size, count) = dir_size_and_count(&g_dir);
                if fs::remove_dir_all(&g_dir).is_ok() {
                    cleaned_bytes += size;
                    cleaned_items += count;
                    println!(
                        "✓ Cleaned global cache: {} ({} freed)",
                        g_dir.display(),
                        format_bytes(size)
                    );
                }
            }
        }

        if let Some(c_dir) = get_global_c_obj_dir() {
            if c_dir.exists() {
                let (size, count) = dir_size_and_count(&c_dir);
                if fs::remove_dir_all(&c_dir).is_ok() {
                    cleaned_bytes += size;
                    cleaned_items += count;
                    println!(
                        "✓ Cleaned C object cache: {} ({} freed)",
                        c_dir.display(),
                        format_bytes(size)
                    );
                }
            }
        }
    }

    // 3. Clean temporary download archives
    let temp_dir = env::temp_dir();
    if let Ok(entries) = fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("alya_pkg_") || name.starts_with("alya_extract_") {
                let p = entry.path();
                if let Ok(meta) = p.metadata() {
                    cleaned_bytes += meta.len();
                    cleaned_items += 1;
                }
                let _ = if p.is_dir() {
                    fs::remove_dir_all(&p)
                } else {
                    fs::remove_file(&p)
                };
            }
        }
    }

    if cleaned_bytes > 0 || cleaned_items > 0 {
        println!(
            "\n✓ Successfully reclaimed {} of disk space.",
            format_bytes(cleaned_bytes)
        );
    } else {
        println!("Package cache is already empty. Nothing to clean.");
    }

    Ok(())
}
