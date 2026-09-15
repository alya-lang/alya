use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use super::cache::{get_global_cache_dir, run_cache, run_clean};
use super::discovery::{find_manifest_dir, find_package_entry};
use super::hash::{compute_cache_key, compute_package_checksum};
use super::lock::{parse_lockfile, serialize_lockfile};
use super::manifest::{check_compiler_compatibility, parse_manifest, serialize_manifest};
use super::resolver::{
    copy_dir_all, fetch_git_or_archive_dependency, resolve_package_spec, resolve_registry_url,
    update_git_dependency,
};
use super::types::{
    DependencySource, LockedPackage, PackageInfo, PackageLock, PackageManifest, PkgCommand,
};

pub fn run_pkg(cmd: &PkgCommand) -> Result<(), String> {
    match cmd {
        PkgCommand::Init { path, name, is_lib } => {
            run_init(path.as_deref(), name.as_deref(), *is_lib)
        }
        PkgCommand::Add {
            name,
            path,
            git,
            tag,
            branch,
            version,
        } => run_add(
            name,
            path.as_deref(),
            git.as_deref(),
            tag.as_deref(),
            branch.as_deref(),
            version.as_deref(),
        ),
        PkgCommand::Install => run_install(),
        PkgCommand::List => run_list(),
        PkgCommand::Update => run_update(),
        PkgCommand::Cache { clean, .. } => {
            if *clean {
                run_clean(true)
            } else {
                run_cache()
            }
        }
        PkgCommand::Clean { all } => run_clean(*all),
        PkgCommand::Help => {
            print_pkg_help();
            Ok(())
        }
    }
}

pub fn run_init(path: Option<&str>, name: Option<&str>, is_lib: bool) -> Result<(), String> {
    let target_dir = PathBuf::from(path.unwrap_or("."));
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|e| {
            format!(
                "Failed to create directory '{}': {}",
                target_dir.display(),
                e
            )
        })?;
    }

    let manifest_path = target_dir.join("alya.toml");
    if manifest_path.exists() {
        return Err(format!(
            "Package manifest '{}' already exists.",
            manifest_path.display()
        ));
    }

    let pkg_name = if let Some(n) = name {
        n.to_string()
    } else {
        let abs = target_dir
            .canonicalize()
            .unwrap_or_else(|_| target_dir.clone());
        abs.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "my_package".to_string())
            .to_lowercase()
            .replace(' ', "_")
    };

    let entry_file = if is_lib {
        "src/lib.alya"
    } else {
        "src/main.alya"
    };

    let manifest = PackageManifest {
        package: PackageInfo {
            name: pkg_name.clone(),
            version: "0.1.0".to_string(),
            alya_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            authors: Vec::new(),
            description: Some(format!("Alya package {}", pkg_name)),
            entry: entry_file.to_string(),
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            keywords: Vec::new(),
            extra: BTreeMap::new(),
        },
        dependencies: BTreeMap::new(),
        build: None,
    };

    fs::write(&manifest_path, serialize_manifest(&manifest))
        .map_err(|e| format!("Failed to write alya.toml: {}", e))?;

    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create 'src' directory: {}", e))?;

    let code_entry = target_dir.join(entry_file);
    if !code_entry.exists() {
        let starter_code = if is_lib {
            format!(
                "# {} library\n\nfunction add(a, b)\n    return a + b\nend\n",
                pkg_name
            )
        } else {
            format!(
                "# {} application\n\nfunction main()\n    say \"Hello from {}!\"\nend\n\nmain()\n",
                pkg_name, pkg_name
            )
        };
        fs::write(&code_entry, starter_code)
            .map_err(|e| format!("Failed to create entry file: {}", e))?;
    }

    let gitignore_path = target_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore = if is_lib {
            "/target/\n.alya/\nalya.lock\nalya.lock.bak\n*.exe\n*.s\n*.o\n*.app\n"
        } else {
            "/target/\n.alya/\nalya.lock.bak\n*.exe\n*.s\n*.o\n*.app\n"
        };
        let _ = fs::write(gitignore_path, gitignore);
    }

    println!(
        "✓ Created {} package '{}' at {}",
        if is_lib { "library" } else { "binary" },
        pkg_name,
        target_dir.display()
    );
    Ok(())
}

pub fn run_add(
    name: &str,
    path: Option<&str>,
    git: Option<&str>,
    tag: Option<&str>,
    branch: Option<&str>,
    version: Option<&str>,
) -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    let manifest_path = manifest_dir.join("alya.toml");

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let mut manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;

    let (resolved_name, auto_git) = resolve_package_spec(name);

    let (source, display_ver) = if let Some(p) = path {
        (
            DependencySource::Path {
                path: p.to_string(),
            },
            None,
        )
    } else if let Some(g) = git {
        (
            DependencySource::Git {
                url: g.to_string(),
                tag: tag.map(|s| s.to_string()),
                branch: branch.map(|s| s.to_string()),
                rev: None,
            },
            tag.or(branch).map(|s| s.to_string()),
        )
    } else if let Some(auto_url) = auto_git {
        (
            DependencySource::Git {
                url: auto_url,
                tag: tag.map(|s| s.to_string()),
                branch: branch.map(|s| s.to_string()),
                rev: None,
            },
            tag.or(branch).map(|s| s.to_string()),
        )
    } else {
        // Short-name / Registry dependency
        let url = resolve_registry_url(&resolved_name);
        let ver = if let Some(v) = version {
            v.to_string()
        } else if let Some(t) = tag {
            t.trim_start_matches(['v', 'V']).to_string()
        } else {
            // Dynamically inspect package for declared version without hardcoding
            let mut detected_ver = None;
            if let Some(global_cache_dir) = get_global_cache_dir() {
                let cache_key = compute_cache_key(&resolved_name, "head", &url);
                let cached_pkg_dir = global_cache_dir.join(&cache_key);
                if !cached_pkg_dir.exists() || !cached_pkg_dir.join("alya.toml").exists() {
                    let _ = fs::create_dir_all(&global_cache_dir);
                    let _ = fetch_git_or_archive_dependency(
                        &resolved_name,
                        &url,
                        None,
                        branch,
                        None,
                        &cached_pkg_dir,
                    );
                }
                if cached_pkg_dir.exists() && !cached_pkg_dir.join(".alya-source").exists() {
                    let _ = fs::write(
                        cached_pkg_dir.join(".alya-source"),
                        format!("git:{}#head", url),
                    );
                }
                if let Ok(manifest_src) = fs::read_to_string(cached_pkg_dir.join("alya.toml")) {
                    if let Ok(parsed) = parse_manifest(&manifest_src) {
                        detected_ver = Some(parsed.package.version);
                    }
                }
            }
            detected_ver.unwrap_or_else(|| "0.1.0".to_string())
        };
        let v_disp = ver.clone();
        (DependencySource::Version(ver), Some(v_disp))
    };

    manifest.dependencies.insert(resolved_name.clone(), source);

    fs::write(&manifest_path, serialize_manifest(&manifest))
        .map_err(|e| format!("Failed to update alya.toml: {}", e))?;

    if let Some(v) = display_ver {
        println!(
            "✓ Added dependency '{}' (v{}) to alya.toml",
            resolved_name, v
        );
    } else {
        println!("✓ Added dependency '{}' to alya.toml", resolved_name);
    }

    run_install_in(&manifest_dir)?;
    Ok(())
}

pub fn run_install() -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    run_install_in(&manifest_dir)
}

pub fn run_install_in(manifest_dir: &Path) -> Result<(), String> {
    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;

    let packages_dir = manifest_dir.join(".alya").join("packages");
    let mut locked_packages = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut to_process: VecDeque<(String, DependencySource, PathBuf)> = VecDeque::new();

    for (name, dep) in &manifest.dependencies {
        to_process.push_back((name.clone(), dep.clone(), manifest_dir.to_path_buf()));
    }

    while let Some((name, dep, from_manifest_dir)) = to_process.pop_front() {
        if visited.contains(&name) {
            continue;
        }
        visited.insert(name.clone());

        let resolved_pkg_dir = match dep {
            DependencySource::Path { path } => {
                let dep_path = Path::new(&path);
                let full_path = if dep_path.is_absolute() {
                    dep_path.to_path_buf()
                } else {
                    from_manifest_dir.join(dep_path)
                };

                if !full_path.exists() {
                    return Err(format!(
                        "Path dependency '{}' does not exist at '{}'",
                        name,
                        full_path.display()
                    ));
                }

                let entry = find_package_entry(&full_path, &name)?;
                let rel_entry = entry
                    .strip_prefix(manifest_dir)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .replace('\\', "/");
                let checksum = compute_package_checksum(&full_path)?;

                locked_packages.push(LockedPackage {
                    name: name.clone(),
                    version: "0.1.0".to_string(),
                    source: format!("path:{}", path.replace('\\', "/")),
                    entry: rel_entry,
                    checksum,
                    dependencies: Vec::new(),
                });

                Some(full_path)
            }
            DependencySource::Git {
                url,
                tag,
                branch,
                rev,
            } => {
                fs::create_dir_all(&packages_dir)
                    .map_err(|e| format!("Failed to create .alya/packages directory: {}", e))?;
                let target_dir = packages_dir.join(&name);

                let tag_or_branch = tag
                    .as_deref()
                    .or(branch.as_deref())
                    .or(rev.as_deref())
                    .unwrap_or("head");
                let source = format!("git:{}#{}", url, tag_or_branch);

                if let Some(global_cache_dir) = get_global_cache_dir() {
                    let cache_key = compute_cache_key(&name, tag_or_branch, &url);
                    let cached_pkg_dir = global_cache_dir.join(&cache_key);

                    let cache_hit =
                        cached_pkg_dir.exists() && cached_pkg_dir.join("alya.toml").exists();
                    if cache_hit {
                        println!(
                            "  Using cached package '{}' ({}) from global cache",
                            name, tag_or_branch
                        );
                        if !target_dir.exists() || !target_dir.join("alya.toml").exists() {
                            if target_dir.exists() {
                                let _ = fs::remove_dir_all(&target_dir);
                            }
                            copy_dir_all(&cached_pkg_dir, &target_dir, true)?;
                        }
                    } else {
                        let _ = fs::create_dir_all(&global_cache_dir);
                        if cached_pkg_dir.exists() {
                            let _ = fs::remove_dir_all(&cached_pkg_dir);
                        }
                        fetch_git_or_archive_dependency(
                            &name,
                            &url,
                            tag.as_deref(),
                            branch.as_deref(),
                            rev.as_deref(),
                            &cached_pkg_dir,
                        )?;
                        let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);

                        if target_dir.exists() {
                            let _ = fs::remove_dir_all(&target_dir);
                        }
                        copy_dir_all(&cached_pkg_dir, &target_dir, true)?;
                    }
                } else if !target_dir.exists() {
                    fetch_git_or_archive_dependency(
                        &name,
                        &url,
                        tag.as_deref(),
                        branch.as_deref(),
                        rev.as_deref(),
                        &target_dir,
                    )?;
                } else {
                    update_git_dependency(tag.as_deref(), branch.as_deref(), &target_dir);
                }

                // Ensure local project dependency directory never contains a .git folder
                let local_git_dir = target_dir.join(".git");
                if local_git_dir.exists() {
                    let _ = fs::remove_dir_all(&local_git_dir);
                }

                let entry = find_package_entry(&target_dir, &name)?;
                let rel_entry = entry
                    .strip_prefix(manifest_dir)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .replace('\\', "/");
                let checksum = compute_package_checksum(&target_dir)?;

                locked_packages.push(LockedPackage {
                    name: name.clone(),
                    version: tag_or_branch.to_string(),
                    source,
                    entry: rel_entry,
                    checksum,
                    dependencies: Vec::new(),
                });

                Some(target_dir)
            }
            DependencySource::Version(v) => {
                fs::create_dir_all(&packages_dir)
                    .map_err(|e| format!("Failed to create .alya/packages directory: {}", e))?;
                let target_dir = packages_dir.join(&name);

                let url = resolve_registry_url(&name);
                let tag_cand = if v != "*" && !v.is_empty() {
                    Some(if v.starts_with('v') || v.starts_with('V') {
                        v.clone()
                    } else {
                        format!("v{}", v)
                    })
                } else {
                    None
                };

                let tag_or_branch = tag_cand.as_deref().unwrap_or("head");
                let source = format!("registry+{}#{}", url, v);

                if let Some(global_cache_dir) = get_global_cache_dir() {
                    let cache_key = compute_cache_key(&name, tag_or_branch, &url);
                    let cached_pkg_dir = global_cache_dir.join(&cache_key);

                    let cache_hit =
                        cached_pkg_dir.exists() && cached_pkg_dir.join("alya.toml").exists();

                    let head_cache_key = compute_cache_key(&name, "head", &url);
                    let head_cached_dir = global_cache_dir.join(&head_cache_key);
                    let head_hit = if !cache_hit
                        && head_cached_dir.exists()
                        && head_cached_dir.join("alya.toml").exists()
                    {
                        if let Ok(manifest_src) =
                            fs::read_to_string(head_cached_dir.join("alya.toml"))
                        {
                            if let Ok(parsed) = parse_manifest(&manifest_src) {
                                parsed.package.version == *v || v == "*"
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if cache_hit {
                        println!(
                            "  Using cached package '{}' ({}) from global cache",
                            name, v
                        );
                        if !target_dir.exists() || !target_dir.join("alya.toml").exists() {
                            if target_dir.exists() {
                                let _ = fs::remove_dir_all(&target_dir);
                            }
                            copy_dir_all(&cached_pkg_dir, &target_dir, true)?;
                        }
                    } else if head_hit {
                        let _ = copy_dir_all(&head_cached_dir, &cached_pkg_dir, true);
                        let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
                        println!("  Using package '{}' (v{}) from global cache", name, v);
                        if target_dir.exists() {
                            let _ = fs::remove_dir_all(&target_dir);
                        }
                        copy_dir_all(&cached_pkg_dir, &target_dir, true)?;
                    } else {
                        let _ = fs::create_dir_all(&global_cache_dir);
                        if cached_pkg_dir.exists() {
                            let _ = fs::remove_dir_all(&cached_pkg_dir);
                        }
                        let mut fetch_res = fetch_git_or_archive_dependency(
                            &name,
                            &url,
                            tag_cand.as_deref(),
                            None,
                            None,
                            &cached_pkg_dir,
                        );
                        if fetch_res.is_err() {
                            // Fallback 1: Check if 'head' in global cache matches the requested version
                            let head_cache_key = compute_cache_key(&name, "head", &url);
                            let head_cached_dir = global_cache_dir.join(&head_cache_key);
                            if head_cached_dir.exists()
                                && head_cached_dir.join("alya.toml").exists()
                            {
                                if let Ok(manifest_src) =
                                    fs::read_to_string(head_cached_dir.join("alya.toml"))
                                {
                                    if let Ok(parsed) = parse_manifest(&manifest_src) {
                                        if parsed.package.version == *v || v == "*" {
                                            println!(
                                                "  Notice: Tag '{}' not found on remote; using matching head version '{}'",
                                                tag_or_branch, parsed.package.version
                                            );
                                            let _ = copy_dir_all(
                                                &head_cached_dir,
                                                &cached_pkg_dir,
                                                true,
                                            );
                                            fetch_res = Ok(());
                                        }
                                    }
                                }
                            }
                            // Fallback 2: Try fetching default branch (head) directly
                            if fetch_res.is_err() {
                                if let Ok(()) = fetch_git_or_archive_dependency(
                                    &name,
                                    &url,
                                    None,
                                    None,
                                    None,
                                    &cached_pkg_dir,
                                ) {
                                    if let Ok(manifest_src) =
                                        fs::read_to_string(cached_pkg_dir.join("alya.toml"))
                                    {
                                        if let Ok(parsed) = parse_manifest(&manifest_src) {
                                            if parsed.package.version == *v || v == "*" {
                                                println!(
                                                    "  Notice: Tag '{}' not found on remote; using matching head version '{}'",
                                                    tag_or_branch, parsed.package.version
                                                );
                                                fetch_res = Ok(());
                                            } else {
                                                let _ = fs::remove_dir_all(&cached_pkg_dir);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if let Err(e) = fetch_res {
                            if !target_dir.exists() || !target_dir.join("alya.toml").exists() {
                                return Err(e);
                            }
                        } else {
                            let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
                            if target_dir.exists() {
                                let _ = fs::remove_dir_all(&target_dir);
                            }
                            copy_dir_all(&cached_pkg_dir, &target_dir, true)?;
                        }
                    }
                } else if !target_dir.exists() || !target_dir.join("alya.toml").exists() {
                    let mut fetch_res = fetch_git_or_archive_dependency(
                        &name,
                        &url,
                        tag_cand.as_deref(),
                        None,
                        None,
                        &target_dir,
                    );
                    if fetch_res.is_err() {
                        if let Ok(()) = fetch_git_or_archive_dependency(
                            &name,
                            &url,
                            None,
                            None,
                            None,
                            &target_dir,
                        ) {
                            if let Ok(manifest_src) =
                                fs::read_to_string(target_dir.join("alya.toml"))
                            {
                                if let Ok(parsed) = parse_manifest(&manifest_src) {
                                    if parsed.package.version == *v || v == "*" {
                                        fetch_res = Ok(());
                                    }
                                }
                            }
                        }
                    }
                    fetch_res?;
                } else {
                    update_git_dependency(tag_cand.as_deref(), None, &target_dir);
                }

                // Ensure local project dependency directory never contains a .git folder
                let local_git_dir = target_dir.join(".git");
                if local_git_dir.exists() {
                    let _ = fs::remove_dir_all(&local_git_dir);
                }

                let entry = find_package_entry(&target_dir, &name)?;
                let rel_entry = entry
                    .strip_prefix(manifest_dir)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .replace('\\', "/");
                let checksum = compute_package_checksum(&target_dir)?;

                let actual_version = if v == "*" || v.is_empty() {
                    if let Ok(content) = fs::read_to_string(target_dir.join("alya.toml")) {
                        if let Ok(pkg_m) = parse_manifest(&content) {
                            pkg_m.package.version
                        } else {
                            v.clone()
                        }
                    } else {
                        v.clone()
                    }
                } else {
                    v.clone()
                };

                locked_packages.push(LockedPackage {
                    name: name.clone(),
                    version: actual_version.clone(),
                    source: format!("registry+{}#v{}", url, actual_version),
                    entry: rel_entry,
                    checksum,
                    dependencies: Vec::new(),
                });

                Some(target_dir)
            }
        };

        // Recursively inspect the resolved package for its own dependencies in alya.toml
        let mut sub_deps = Vec::new();
        if let Some(pkg_dir) = resolved_pkg_dir {
            let sub_manifest_path = pkg_dir.join("alya.toml");
            if sub_manifest_path.exists() {
                if let Ok(sub_content) = fs::read_to_string(&sub_manifest_path) {
                    if let Ok(sub_manifest) = parse_manifest(&sub_content) {
                        for (sub_name, sub_dep) in sub_manifest.dependencies {
                            sub_deps.push(sub_name.clone());
                            if !visited.contains(&sub_name) {
                                to_process.push_back((sub_name, sub_dep, pkg_dir.clone()));
                            }
                        }
                    }
                }
            }
        }
        sub_deps.sort();
        sub_deps.dedup();
        if let Some(pkg) = locked_packages.last_mut() {
            pkg.dependencies = sub_deps;
        }
    }

    let lock = PackageLock {
        version: 1,
        packages: locked_packages,
    };

    let lockfile_path = manifest_dir.join("alya.lock");
    fs::write(&lockfile_path, serialize_lockfile(&lock))
        .map_err(|e| format!("Failed to write alya.lock: {}", e))?;

    // On case-sensitive filesystems, remove legacy Alya.lock if distinct from alya.lock
    let legacy_lock = manifest_dir.join("Alya.lock");
    if legacy_lock.exists() {
        if let (Ok(p1), Ok(p2)) = (lockfile_path.canonicalize(), legacy_lock.canonicalize()) {
            if p1 != p2 {
                let _ = fs::remove_file(&legacy_lock);
            }
        }
    }

    println!(
        "✓ Locked {} package{} in alya.lock",
        lock.packages.len(),
        if lock.packages.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

pub fn run_list() -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let manifest = parse_manifest(&content)?;

    println!(
        "Package: {} v{}",
        manifest.package.name, manifest.package.version
    );
    println!("Entry:   {}", manifest.package.entry);
    if let Some(desc) = &manifest.package.description {
        println!("About:   {}", desc);
    }
    println!();

    if manifest.dependencies.is_empty() {
        println!("No dependencies declared in alya.toml.");
        return Ok(());
    }

    println!("Dependencies ({}):", manifest.dependencies.len());

    let lock_path = if manifest_dir.join("alya.lock").exists() {
        manifest_dir.join("alya.lock")
    } else {
        manifest_dir.join("Alya.lock")
    };
    let lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };

    for (name, dep) in &manifest.dependencies {
        let locked = lock
            .as_ref()
            .and_then(|l| l.packages.iter().find(|p| &p.name == name));
        let dep_desc = match dep {
            DependencySource::Path { path } => format!("path: {}", path),
            DependencySource::Git {
                url, tag, branch, ..
            } => {
                let mut s = format!("git: {}", url);
                if let Some(t) = tag {
                    s.push_str(&format!(" (tag: {})", t));
                } else if let Some(b) = branch {
                    s.push_str(&format!(" (branch: {})", b));
                }
                s
            }
            DependencySource::Version(v) => format!("version: {}", v),
        };

        if let Some(lp) = locked {
            let chk_short = if lp.checksum.len() > 17 {
                &lp.checksum[..17]
            } else {
                &lp.checksum
            };
            println!(
                "  • {:<16} {:<35} [locked: {}...]",
                name, dep_desc, chk_short
            );
        } else {
            println!(
                "  • {:<16} {:<35} [not locked - run 'alyac install']",
                name, dep_desc
            );
        }
    }

    if let Some(ref l) = lock {
        let transitive: Vec<_> = l
            .packages
            .iter()
            .filter(|p| !manifest.dependencies.contains_key(&p.name))
            .collect();
        if !transitive.is_empty() {
            println!("\nTransitive Dependencies ({}):", transitive.len());
            for p in transitive {
                let chk_short = if p.checksum.len() > 17 {
                    &p.checksum[..17]
                } else {
                    &p.checksum
                };
                let src_short = if p.source.len() > 34 {
                    format!("{}...", &p.source[..31])
                } else {
                    p.source.clone()
                };
                println!(
                    "  • {:<16} {:<35} [locked: {}...]",
                    p.name, src_short, chk_short
                );
            }
        }
    }

    Ok(())
}

pub fn run_update() -> Result<(), String> {
    println!("Updating package dependencies...");
    run_install()
}

pub fn print_pkg_help() {
    println!("Alya Package Manager (alyac pkg)");
    println!("Manage project dependencies, manifests (alya.toml), and lockfiles (alya.lock).\n");
    println!("USAGE:");
    println!("  alyac pkg <COMMAND> [OPTIONS]");
    println!("  alyac init [path] [OPTIONS]          # Shortcut for pkg init");
    println!("  alyac add <name> [OPTIONS]           # Shortcut for pkg add");
    println!("  alyac install                        # Shortcut for pkg install");
    println!("  alyac cache                          # Shortcut for pkg cache");
    println!("  alyac clean                          # Shortcut for pkg clean\n");
    println!("COMMANDS:");
    println!("  init [path]        Initialize a new Alya package in [path] (default: .)");
    println!("  add <name>         Add a new dependency to alya.toml");
    println!("  install            Resolve and lock all dependencies specified in alya.toml");
    println!("  list               List project dependencies and lock integrity status");
    println!("  update             Update and re-lock dependencies to latest versions");
    println!("  cache [clean]      Inspect package cache directory, size, and installed packages");
    println!("  clean [--all]      Remove cached dependencies and reclaim disk space");
    println!("  help               Show this help message\n");
    println!("OPTIONS FOR 'init':");
    println!("  --name <name>      Override package name (default: directory name)");
    println!("  --lib              Initialize as a library (src/lib.alya) instead of binary\n");
    println!("OPTIONS FOR 'add':");
    println!("  --path <path>      Add dependency from local file system path");
    println!("  --git <url>        Add dependency from remote Git repository");
    println!("  --tag <tag>        Specify Git tag for dependency");
    println!("  --branch <branch>  Specify Git branch for dependency");
    println!("  --version <ver>    Specify semantic version constraint");
    println!("  Package name supports:");
    println!("    • Short-name:     http                 (resolves to alya-lang/http)");
    println!("    • Versioned:      http@0.1.0           (resolves to alya-lang/http tag v0.1.0)");
    println!("    • GitHub repo:    owner/repo           (resolves to https://github.com/owner/repo.git)\n");
    println!("OPTIONS FOR 'cache':");
    println!("  clean, --clean     Clean cached packages and reclaim disk space");
    println!("  --all, -a          Inspect or clean both local and global cache\n");
    println!("OPTIONS FOR 'clean':");
    println!("  --all, -a          Clean both local project packages and global cache\n");
    println!("EXAMPLES:");
    println!("  alyac init my_app");
    println!("  alyac init my_lib --lib");
    println!("  alyac add http                       # Add official package via short-name");
    println!("  alyac add crypto@0.1.0               # Add official package with version");
    println!("  alyac add raylib --path ../libs/raylib");
    println!("  alyac add sqlite --git https://github.com/alya-lang/sqlite --tag v1.0.0");
    println!("  alyac install");
    println!("  alyac pkg list");
    println!("  alyac pkg cache");
    println!("  alyac pkg clean");
}
