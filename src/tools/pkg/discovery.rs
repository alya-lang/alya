use super::manifest::{check_compiler_compatibility, parse_manifest};
use super::types::DependencySource;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_manifest_dir() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    find_manifest_dir_from(&cwd)
}

pub fn find_manifest_dir_from(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("alya.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

pub fn detect_package_entry() -> Option<String> {
    let manifest_dir = find_manifest_dir()?;
    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path).ok()?;
    let manifest = parse_manifest(&content).ok()?;
    if let Err(e) = check_compiler_compatibility(&manifest) {
        eprintln!("Error: {}", e);
        return None;
    }
    let entry_path = manifest_dir.join(&manifest.package.entry);
    if entry_path.exists() {
        Some(entry_path.to_string_lossy().replace('\\', "/"))
    } else {
        None
    }
}

pub fn collect_alya_files(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    if !current.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(current)
        .map_err(|e| format!("Cannot read directory '{}': {}", current.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == ".git" || file_name == ".alya" || file_name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_alya_files(root, &path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "alya") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, path));
        }
    }
    Ok(())
}

pub fn find_package_entry(pkg_dir: &Path, pkg_name: &str) -> Result<PathBuf, String> {
    let manifest_file = pkg_dir.join("alya.toml");
    if manifest_file.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_file) {
            if let Ok(manifest) = parse_manifest(&content) {
                check_compiler_compatibility(&manifest)?;
                let candidate = pkg_dir.join(&manifest.package.entry);
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }
    }

    let candidates = [
        pkg_dir.join("src").join("main.alya"),
        pkg_dir.join("src").join("lib.alya"),
        pkg_dir.join("main.alya"),
        pkg_dir.join("lib.alya"),
        pkg_dir.join(format!("{}.alya", pkg_name)),
        pkg_dir.join("src").join(format!("{}.alya", pkg_name)),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    Err(format!(
        "Cannot find entry point for package '{}' in directory '{}'. Expected 'src/main.alya', 'src/lib.alya', or manifest entry.",
        pkg_name,
        pkg_dir.display()
    ))
}

pub fn resolve_package_import(
    import_path: &str,
    current_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    if import_path.starts_with("./")
        || import_path.starts_with("../")
        || import_path.starts_with('/')
        || import_path.starts_with('\\')
        || import_path.starts_with("std/")
        || import_path.starts_with("std::")
    {
        return Ok(None);
    }

    let manifest_dir = match find_manifest_dir_from(current_dir) {
        Some(d) => d,
        None => return Ok(None),
    };

    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read 'alya.toml': {}", e))?;
    let manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;

    let parts: Vec<&str> = import_path.splitn(2, '/').collect();
    let pkg_name = parts[0];
    let subpath = if parts.len() > 1 {
        Some(parts[1])
    } else {
        None
    };

    if let Some(dep_source) = manifest.dependencies.get(pkg_name) {
        let pkg_dir = match dep_source {
            DependencySource::Path { path } => {
                let p = Path::new(path);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    manifest_dir.join(p)
                }
            }
            DependencySource::Git { .. } | DependencySource::Version(_) => {
                let local_pkg_dir = manifest_dir.join(".alya").join("packages").join(pkg_name);
                if local_pkg_dir.exists() {
                    local_pkg_dir
                } else {
                    let mut found = None;
                    let mut curr = manifest_dir.parent();
                    while let Some(p) = curr {
                        let candidate = p.join(".alya").join("packages").join(pkg_name);
                        if candidate.exists() {
                            found = Some(candidate);
                            break;
                        }
                        curr = p.parent();
                    }
                    if found.is_none() {
                        if let Some(root_manifest) = find_manifest_dir() {
                            let candidate =
                                root_manifest.join(".alya").join("packages").join(pkg_name);
                            if candidate.exists() {
                                found = Some(candidate);
                            }
                        }
                    }
                    found.unwrap_or(local_pkg_dir)
                }
            }
        };

        if !pkg_dir.exists() {
            return Err(format!(
                "Package '{}' is declared in alya.toml but not installed at '{}'. Run 'alya install' to resolve dependencies.",
                pkg_name,
                pkg_dir.display()
            ));
        }

        if let Some(sub) = subpath {
            let candidates = [
                pkg_dir.join("src").join(format!("{}.alya", sub)),
                pkg_dir.join("src").join(sub).join("mod.alya"),
                pkg_dir.join("src").join(sub),
                pkg_dir.join(format!("{}.alya", sub)),
                pkg_dir.join(sub).join("mod.alya"),
                pkg_dir.join(sub),
            ];
            for cand in candidates {
                if cand.exists() {
                    return Ok(Some(cand));
                }
            }
            return Err(format!(
                "Cannot find module '{}' in package '{}' (searched inside '{}')",
                sub,
                pkg_name,
                pkg_dir.display()
            ));
        } else {
            let entry = find_package_entry(&pkg_dir, pkg_name)?;
            return Ok(Some(entry));
        }
    }

    Ok(None)
}
