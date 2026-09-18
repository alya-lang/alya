use crate::codegen::{Architecture, OperatingSystem};
use crate::tools::pkg::cache::get_global_c_obj_dir;
use crate::tools::pkg::discovery::find_manifest_dir_from;
use crate::tools::pkg::manifest::parse_manifest;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static C_BUILD_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default)]
pub struct CBuildPlan {
    pub sources: Vec<PathBuf>,
    pub flags: Vec<String>,
    pub include_dirs: Vec<PathBuf>,
    pub provided_libs: HashSet<String>,
}

/// Discovers all C sources declared in `[build]` sections of `alya.toml`
/// for the main project file and all imported files.
pub fn discover_c_build_plan(
    main_file: &Path,
    imported_files: &HashSet<PathBuf>,
) -> Result<CBuildPlan, String> {
    let mut plan = CBuildPlan::default();
    let mut checked_manifest_dirs = HashSet::new();

    // Collect all directories to search for alya.toml:
    let mut search_dirs = Vec::new();
    if let Some(parent) = main_file.parent() {
        search_dirs.push(parent.to_path_buf());
    } else {
        search_dirs.push(PathBuf::from("."));
    }

    for file in imported_files {
        if let Some(parent) = file.parent() {
            search_dirs.push(parent.to_path_buf());
        }
    }

    for dir in search_dirs {
        if let Some(manifest_dir) = find_manifest_dir_from(&dir) {
            let canon_manifest_dir = clean_canonicalize(&manifest_dir);
            if checked_manifest_dirs.insert(canon_manifest_dir.clone()) {
                let manifest_file = canon_manifest_dir.join("alya.toml");
                if let Ok(content) = fs::read_to_string(&manifest_file) {
                    if let Ok(manifest) = parse_manifest(&content) {
                        if let Some(build) = manifest.build {
                            for src in build.c_sources {
                                let src_path = canon_manifest_dir.join(&src);
                                let canon_src = clean_canonicalize(&src_path);
                                if !plan.sources.contains(&canon_src) {
                                    plan.sources.push(canon_src);
                                }
                            }
                            for flag in build.c_flags {
                                if !plan.flags.contains(&flag) {
                                    plan.flags.push(flag);
                                }
                            }
                            for inc in build.c_include_dirs {
                                let inc_path = canon_manifest_dir.join(&inc);
                                let canon_inc = clean_canonicalize(&inc_path);
                                if !plan.include_dirs.contains(&canon_inc) {
                                    plan.include_dirs.push(canon_inc);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Determine provided libraries by stem name of C sources (e.g. sqlite3.c provides "sqlite3" and "sqlite")
    for src in &plan.sources {
        if let Some(stem) = src.file_stem().and_then(|s| s.to_str()) {
            plan.provided_libs.insert(stem.to_string());
            let stripped = stem.trim_end_matches(|c: char| c.is_ascii_digit());
            if !stripped.is_empty() {
                plan.provided_libs.insert(stripped.to_string());
            }
        }
    }

    Ok(plan)
}

/// Compiles C source files into object files (.o) with disk caching in ~/.alya/cache/c_obj.
/// Reuses cached .o files if the source file mtime, size, and compiler flags haven't changed.
pub fn build_c_objects(
    plan: &CBuildPlan,
    arch: Architecture,
    os: OperatingSystem,
) -> Result<Vec<PathBuf>, String> {
    if plan.sources.is_empty() {
        return Ok(Vec::new());
    }

    let _lock = C_BUILD_MUTEX.lock().unwrap();

    let cache_dir = get_global_c_obj_dir().unwrap_or_else(|| PathBuf::from(".alya").join("c_obj"));
    let cache_dir = clean_canonicalize(&cache_dir);

    fs::create_dir_all(&cache_dir).map_err(|e| {
        format!(
            "Failed to create C object cache directory '{}': {}",
            cache_dir.display(),
            e
        )
    })?;

    let mut object_files = Vec::new();

    for src in &plan.sources {
        if !src.exists() {
            return Err(format!(
                "Declared C source file not found: '{}'",
                src.display()
            ));
        }

        let canon_src = clean_canonicalize(src);
        let stem = canon_src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("obj");
        let meta = fs::metadata(&canon_src)
            .map_err(|e| format!("Cannot read metadata for '{}': {}", canon_src.display(), e))?;
        let mtime = meta
            .modified()
            .map(|t| format!("{:?}", t))
            .unwrap_or_default();
        let size = meta.len();

        // Compute hash for cache key
        let mut key_data = format!(
            "{}:{}:{}:{:?}:{:?}",
            canon_src.display(),
            mtime,
            size,
            arch,
            os
        );
        for f in &plan.flags {
            key_data.push_str(f);
        }
        for inc in &plan.include_dirs {
            key_data.push_str(&inc.display().to_string());
        }

        let hash_val = format!("{:016x}", calculate_hash(&key_data));
        let obj_name = format!("{}_{}.o", stem, hash_val);
        let obj_path = cache_dir.join(&obj_name);

        if !obj_path.exists() || fs::metadata(&obj_path).map(|m| m.len()).unwrap_or(0) == 0 {
            let mut gcc_args = vec![
                "-c".to_string(),
                path_to_gcc_arg(&canon_src),
                "-o".to_string(),
                path_to_gcc_arg(&obj_path),
            ];

            if matches!(arch, Architecture::X86) {
                gcc_args.push("-m32".to_string());
            }

            // Enable section-level dead code elimination for C sources
            gcc_args.push("-ffunction-sections".to_string());
            gcc_args.push("-fdata-sections".to_string());

            for inc in &plan.include_dirs {
                gcc_args.push(format!("-I{}", path_to_gcc_arg(inc)));
            }

            for flag in &plan.flags {
                gcc_args.push(flag.clone());
            }

            let toolchain = crate::driver::toolchain::resolve_toolchain(arch, os, false)?;
            let output = Command::new(&toolchain.compiler_path)
                .args(&gcc_args)
                .output()
                .map_err(|e| {
                    format!(
                        "Failed to invoke compiler '{}' to compile C source '{}': {}",
                        toolchain.compiler_path.display(),
                        canon_src.display(),
                        e
                    )
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "Compiler ({}) failed to compile C source '{}':\n{}",
                    toolchain.compiler_path.display(),
                    canon_src.display(),
                    stderr
                ));
            }
        }

        let clean_obj_path = clean_canonicalize(&obj_path);
        if !object_files.contains(&clean_obj_path) {
            object_files.push(clean_obj_path);
        }
    }

    Ok(object_files)
}

pub fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", rest))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

pub fn clean_canonicalize(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(p) => strip_unc_prefix(&p),
        Err(_) => strip_unc_prefix(path),
    }
}

pub fn path_to_gcc_arg(path: &Path) -> String {
    let cleaned = strip_unc_prefix(path);
    cleaned.to_string_lossy().replace('\\', "/")
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_unc_prefix() {
        let unc = Path::new(r"\\?\D:\a\sqlite\sqlite\c\sqlite3.c");
        assert_eq!(
            strip_unc_prefix(unc),
            PathBuf::from(r"D:\a\sqlite\sqlite\c\sqlite3.c")
        );

        let unc_share = Path::new(r"\\?\UNC\server\share\c\sqlite3.c");
        assert_eq!(
            strip_unc_prefix(unc_share),
            PathBuf::from(r"\\server\share\c\sqlite3.c")
        );

        let normal = Path::new(r"D:\a\sqlite\sqlite\c\sqlite3.c");
        assert_eq!(
            strip_unc_prefix(normal),
            PathBuf::from(r"D:\a\sqlite\sqlite\c\sqlite3.c")
        );
    }

    #[test]
    fn test_path_to_gcc_arg() {
        let unc = Path::new(r"\\?\D:\a\sqlite\sqlite\c\sqlite3.c");
        assert_eq!(path_to_gcc_arg(unc), "D:/a/sqlite/sqlite/c/sqlite3.c");

        let normal = Path::new(r"D:\project\file.c");
        assert_eq!(path_to_gcc_arg(normal), "D:/project/file.c");

        let unix = Path::new("/usr/local/include/sqlite3.h");
        assert_eq!(path_to_gcc_arg(unix), "/usr/local/include/sqlite3.h");
    }
}
