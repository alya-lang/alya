use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use super::cache::{get_global_cache_dir, run_cache, run_clean};
use super::discovery::{find_manifest_dir, find_package_entry};
use super::hash::{compute_cache_key, compute_cache_key_rev, compute_package_checksum};
use super::lock::{format_git_source, parse_git_source_rev, parse_lockfile, serialize_lockfile};
use super::manifest::{check_compiler_compatibility, parse_manifest, serialize_manifest};
use super::resolver::{
    coalesce_semver_versions, compare_semver, copy_dir_all, fetch_git_or_archive_dependency,
    find_latest_semver_tag, query_remote_branch_head, query_remote_tags, query_tag_rev,
    resolve_package_spec, resolve_registry_url, semver_major,
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
        PkgCommand::Update { upgrade } => run_update(*upgrade),
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
            links: None,
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
        section_extras: BTreeMap::new(),
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

fn get_dep_major(dep: &DependencySource, from_dir: &Path) -> Option<u64> {
    match dep {
        DependencySource::Version(v) => semver_major(v),
        DependencySource::Git { tag: Some(t), .. } => semver_major(t),
        DependencySource::Path { path } => {
            let p = Path::new(path);
            let full = if p.is_absolute() {
                p.to_path_buf()
            } else {
                from_dir.join(p)
            };
            if let Ok(c) = fs::read_to_string(full.join("alya.toml")) {
                if let Ok(m) = parse_manifest(&c) {
                    return semver_major(&m.package.version);
                }
            }
            None
        }
        _ => None,
    }
}

fn ensure_dep_cached(
    name: &str,
    dep: &DependencySource,
    from_manifest_dir: &Path,
    existing_lock: Option<&PackageLock>,
    reported: &mut HashSet<String>,
) -> Result<PathBuf, String> {
    match dep {
        DependencySource::Path { path } => {
            let p = Path::new(path);
            let full_path = if p.is_absolute() {
                p.to_path_buf()
            } else {
                from_manifest_dir.join(p)
            };
            if !full_path.exists() {
                return Err(format!(
                    "Path dependency '{}' does not exist at '{}'",
                    name,
                    full_path.display()
                ));
            }
            Ok(full_path)
        }
        DependencySource::Git {
            url,
            tag,
            branch,
            rev,
        } => {
            let locked_rev = existing_lock
                .as_ref()
                .and_then(|l| l.packages.iter().find(|p| p.name == name))
                .and_then(|p| parse_git_source_rev(&p.source));
            let effective_rev = rev.clone().or(locked_rev);

            let tag_or_branch = tag
                .as_deref()
                .or(branch.as_deref())
                .or(effective_rev.as_deref())
                .unwrap_or("head");

            // Resolve moved tags to their current commit so the cache key
            // below is revision-scoped. Lock-pinned revs win (deterministic,
            // offline-safe); a live lookup happens only for fresh resolves,
            // and any failure silently falls back to the legacy tag-only key.
            let resolved_tag_rev: Option<String> = if effective_rev.is_none() {
                tag.as_deref().and_then(|t| query_tag_rev(url, t))
            } else {
                None
            };
            let key_rev = effective_rev.as_deref().or(resolved_tag_rev.as_deref());
            let cache_dir = get_global_cache_dir()
                .unwrap_or_else(|| from_manifest_dir.join(".alya").join("cache"));
            let cache_key = compute_cache_key_rev(name, tag_or_branch, url, key_rev);
            let cached_pkg_dir = cache_dir.join(&cache_key);

            let cache_hit = cached_pkg_dir.exists()
                && cached_pkg_dir.join("alya.toml").exists()
                && (effective_rev.is_none()
                    || fs::read_to_string(cached_pkg_dir.join(".alya-rev"))
                        .ok()
                        .map(|s| s.trim().to_string())
                        == effective_rev);

            if cache_hit {
                if reported.insert(format!("{}:{}", name, tag_or_branch)) {
                    println!(
                        "  Using cached package '{}' ({}) from global cache",
                        name, tag_or_branch
                    );
                }
            } else {
                let _ = fs::create_dir_all(&cache_dir);
                if cached_pkg_dir.exists() {
                    let _ = fs::remove_dir_all(&cached_pkg_dir);
                }
                fetch_git_or_archive_dependency(
                    name,
                    url,
                    tag.as_deref(),
                    branch.as_deref(),
                    effective_rev.as_deref(),
                    &cached_pkg_dir,
                )?;
                let commit_sha = fs::read_to_string(cached_pkg_dir.join(".alya-rev"))
                    .ok()
                    .map(|s| s.trim().to_string())
                    .or_else(|| effective_rev.clone());
                let locked_source = format_git_source(
                    url,
                    branch.as_deref(),
                    tag.as_deref(),
                    rev.as_deref(),
                    commit_sha.as_deref(),
                );
                let _ = fs::write(cached_pkg_dir.join(".alya-source"), &locked_source);
            }
            Ok(cached_pkg_dir)
        }
        DependencySource::Version(v) => {
            let url = resolve_registry_url(name);
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
            let source = format!("registry+{}#{}", url, tag_or_branch);

            let cache_dir = get_global_cache_dir()
                .unwrap_or_else(|| from_manifest_dir.join(".alya").join("cache"));
            let cache_key = compute_cache_key(name, tag_or_branch, &url);
            let cached_pkg_dir = cache_dir.join(&cache_key);

            let cache_hit = cached_pkg_dir.exists() && cached_pkg_dir.join("alya.toml").exists();

            let head_cache_key = compute_cache_key(name, "head", &url);
            let head_cached_dir = cache_dir.join(&head_cache_key);
            let head_hit = if !cache_hit
                && head_cached_dir.exists()
                && head_cached_dir.join("alya.toml").exists()
            {
                if let Ok(manifest_src) = fs::read_to_string(head_cached_dir.join("alya.toml")) {
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
                if reported.insert(format!("{}:{}", name, v)) {
                    println!(
                        "  Using cached package '{}' ({}) from global cache",
                        name, v
                    );
                }
            } else if head_hit {
                let _ = copy_dir_all(&head_cached_dir, &cached_pkg_dir, true);
                let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
                if reported.insert(format!("{}:{}", name, v)) {
                    println!("  Using package '{}' (v{}) from global cache", name, v);
                }
            } else {
                let _ = fs::create_dir_all(&cache_dir);
                if cached_pkg_dir.exists() {
                    let _ = fs::remove_dir_all(&cached_pkg_dir);
                }
                let mut fetch_res = fetch_git_or_archive_dependency(
                    name,
                    &url,
                    tag_cand.as_deref(),
                    None,
                    None,
                    &cached_pkg_dir,
                );
                if fetch_res.is_err() {
                    if head_cached_dir.exists() && head_cached_dir.join("alya.toml").exists() {
                        if let Ok(manifest_src) =
                            fs::read_to_string(head_cached_dir.join("alya.toml"))
                        {
                            if let Ok(parsed) = parse_manifest(&manifest_src) {
                                if parsed.package.version == *v || v == "*" {
                                    let _ = copy_dir_all(&head_cached_dir, &cached_pkg_dir, true);
                                    fetch_res = Ok(());
                                }
                            }
                        }
                    }
                    if fetch_res.is_err() {
                        if let Ok(()) = fetch_git_or_archive_dependency(
                            name,
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
                                        fetch_res = Ok(());
                                    } else {
                                        let _ = fs::remove_dir_all(&cached_pkg_dir);
                                    }
                                }
                            }
                        }
                    }
                }
                fetch_res?;
                let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
            }
            Ok(cached_pkg_dir)
        }
    }
}

pub fn run_install_in(manifest_dir: &Path) -> Result<(), String> {
    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;

    let packages_dir = manifest_dir.join(".alya").join("packages");
    let lock_path = if manifest_dir.join("alya.lock").exists() {
        manifest_dir.join("alya.lock")
    } else {
        manifest_dir.join("Alya.lock")
    };
    let existing_lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };

    // Stage 1: Dependency Graph Discovery & SemVer Coalescing
    let mut resolved_requests: BTreeMap<(String, Option<u64>), (DependencySource, PathBuf)> =
        BTreeMap::new();
    let mut to_scan: VecDeque<(String, DependencySource, PathBuf)> = VecDeque::new();
    let mut reported: HashSet<String> = HashSet::new();

    for (name, dep) in &manifest.dependencies {
        to_scan.push_back((name.clone(), dep.clone(), manifest_dir.to_path_buf()));
    }

    while let Some((name, dep, from_manifest_dir)) = to_scan.pop_front() {
        let maj = get_dep_major(&dep, &from_manifest_dir);
        let key = (name.clone(), maj);

        if let Some((existing_dep, _)) = resolved_requests.get_mut(&key) {
            if let (DependencySource::Version(v1), DependencySource::Version(v2)) =
                (&existing_dep, &dep)
            {
                let coalesced = coalesce_semver_versions(v1, v2)?;
                if coalesced != v1.as_str() {
                    *existing_dep = DependencySource::Version(coalesced.to_string());
                }
            }
            continue;
        }

        resolved_requests.insert(key, (dep.clone(), from_manifest_dir.clone()));

        // Inspect sub-dependencies
        if let Ok(source_dir) = ensure_dep_cached(
            &name,
            &dep,
            &from_manifest_dir,
            existing_lock.as_ref(),
            &mut reported,
        ) {
            let sub_manifest_path = source_dir.join("alya.toml");
            if sub_manifest_path.exists() {
                if let Ok(sub_content) = fs::read_to_string(&sub_manifest_path) {
                    if let Ok(sub_manifest) = parse_manifest(&sub_content) {
                        for (sub_name, sub_dep) in sub_manifest.dependencies {
                            to_scan.push_back((sub_name, sub_dep, source_dir.clone()));
                        }
                    }
                }
            }
        }
    }

    // Stage 2: Major Segregation Analysis
    let mut major_counts: BTreeMap<String, HashSet<Option<u64>>> = BTreeMap::new();
    for (name, maj) in resolved_requests.keys() {
        major_counts.entry(name.clone()).or_default().insert(*maj);
    }
    let multi_major_pkgs: HashSet<String> = major_counts
        .into_iter()
        .filter(|(_, set)| set.len() > 1)
        .map(|(name, _)| name)
        .collect();

    // Stage 3: Installation & Native C-FFI Safety Validation
    fs::create_dir_all(&packages_dir)
        .map_err(|e| format!("Failed to create .alya/packages directory: {}", e))?;

    let mut locked_packages = Vec::new();
    let mut links_tracker: BTreeMap<String, String> = BTreeMap::new();

    for ((name, maj), (dep, from_manifest_dir)) in resolved_requests {
        let is_multi = multi_major_pkgs.contains(&name);
        let folder_name = if is_multi {
            format!("{}-v{}", name, maj.unwrap_or(1))
        } else {
            name.clone()
        };

        let cached_source_dir = ensure_dep_cached(
            &name,
            &dep,
            &from_manifest_dir,
            existing_lock.as_ref(),
            &mut reported,
        )?;
        let is_path_dep = matches!(dep, DependencySource::Path { .. });

        let (target_dir, actual_source_str) = if is_path_dep && !is_multi {
            let path_str = match &dep {
                DependencySource::Path { path } => path.replace('\\', "/"),
                _ => "".to_string(),
            };
            (cached_source_dir.clone(), format!("path:{}", path_str))
        } else {
            let dest_dir = packages_dir.join(&folder_name);
            if dest_dir.exists() {
                let _ = fs::remove_dir_all(&dest_dir);
            }
            copy_dir_all(&cached_source_dir, &dest_dir, true)?;

            let local_git_dir = dest_dir.join(".git");
            if local_git_dir.exists() {
                let _ = fs::remove_dir_all(&local_git_dir);
            }

            // Verify content integrity against a previous lock when one pins
            // this package: recompute over the installed tree and reject
            // tampered or unexpectedly swapped checkouts.
            if let Some(locked) = existing_lock.as_ref().and_then(|l| {
                l.packages
                    .iter()
                    .find(|p| p.name == folder_name)
                    .or_else(|| l.packages.iter().find(|p| p.name == *name))
            }) {
                if !locked.checksum.is_empty() {
                    let actual = compute_package_checksum(&dest_dir)?;
                    if actual != locked.checksum {
                        return Err(format!(
                            "Checksum mismatch for '{}': installed content does not match alya.lock (locked {}, got {}). Delete the lock or reinstall to proceed.",
                            name, locked.checksum, actual
                        ));
                    }
                }
            }

            let source_str = match &dep {
                DependencySource::Version(v) => {
                    let url = resolve_registry_url(&name);
                    let v_tag = if v.starts_with('v') || v.starts_with('V') {
                        v.clone()
                    } else {
                        format!("v{}", v)
                    };
                    format!("registry+{}#{}", url, v_tag)
                }
                DependencySource::Path { path } => {
                    format!("path:{}", path.replace('\\', "/"))
                }
                DependencySource::Git {
                    url,
                    branch,
                    tag,
                    rev,
                } => fs::read_to_string(cached_source_dir.join(".alya-source"))
                    .ok()
                    .unwrap_or_else(|| {
                        format_git_source(
                            url,
                            branch.as_deref(),
                            tag.as_deref(),
                            rev.as_deref(),
                            None,
                        )
                    }),
            };

            (dest_dir, source_str)
        };

        // Parse manifest to check Native C-FFI links and extract metadata
        let sub_manifest_path = target_dir.join("alya.toml");
        let (actual_version, sub_deps) = if sub_manifest_path.exists() {
            let sub_content = fs::read_to_string(&sub_manifest_path).map_err(|e| {
                format!(
                    "Failed to read manifest in '{}': {}",
                    target_dir.display(),
                    e
                )
            })?;
            let sub_manifest = parse_manifest(&sub_content)?;

            // Native C-FFI link conflict detection
            if let Some(links_id) = sub_manifest.links() {
                if let Some(prev) = links_tracker.get(links_id) {
                    if prev != &folder_name {
                        return Err(format!(
                            "Error: Duplicate native C library link '{}' required by both '{}' and '{}'. Align dependency versions to resolve.",
                            links_id, prev, folder_name
                        ));
                    }
                } else {
                    links_tracker.insert(links_id.to_string(), folder_name.clone());
                }
            }

            let mut deps: Vec<String> = sub_manifest.dependencies.keys().cloned().collect();
            deps.sort();
            (sub_manifest.package.version, deps)
        } else {
            ("0.1.0".to_string(), Vec::new())
        };

        let entry = find_package_entry(&target_dir, &name)?;
        let rel_entry = entry
            .strip_prefix(manifest_dir)
            .unwrap_or(&entry)
            .to_string_lossy()
            .replace('\\', "/");
        let checksum = compute_package_checksum(&target_dir)?;

        locked_packages.push(LockedPackage {
            name: folder_name,
            version: actual_version,
            source: actual_source_str,
            entry: rel_entry,
            checksum,
            dependencies: sub_deps,
        });
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
            let integrity = if lp.checksum.is_empty() {
                "integrity: unchecked"
            } else {
                let installed_dir = manifest_dir.join(".alya").join("packages").join(name);
                if !installed_dir.exists() {
                    "integrity: not installed"
                } else {
                    match compute_package_checksum(&installed_dir) {
                        Ok(actual) if actual == lp.checksum => "integrity: ok",
                        Ok(_) => "integrity: MISMATCH",
                        Err(_) => "integrity: unreadable",
                    }
                }
            };
            println!(
                "  • {:<16} {:<35} [locked: {}..., {}]",
                name, dep_desc, chk_short, integrity
            );
        } else {
            println!(
                "  • {:<16} {:<35} [not locked - run 'alya install']",
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

struct UpdateRow {
    name: String,
    current: String,
    latest: String,
    status: String,
    can_upgrade: bool,
    new_source: Option<DependencySource>,
    clear_cache_key: Option<String>,
}

/// Best-effort current revision of an installed/locked git dependency.
///
/// Prefers the lockfile pin, then the local `.alya/packages` checkout.
/// Returns `None` when nothing is installed or locked yet.
fn current_dep_rev(manifest_dir: &Path, lock: &Option<PackageLock>, name: &str) -> Option<String> {
    if let Some(l) = lock {
        if let Some(p) = l.packages.iter().find(|p| p.name == name) {
            if let Some(sha) = parse_git_source_rev(&p.source) {
                return Some(sha);
            }
        }
    }
    fs::read_to_string(
        manifest_dir
            .join(".alya")
            .join("packages")
            .join(name)
            .join(".alya-rev"),
    )
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
}

pub fn run_update(upgrade: bool) -> Result<(), String> {
    let manifest_dir = match find_manifest_dir() {
        Some(d) => d,
        None => {
            return Err(
                "No 'alya.toml' found. Please run this command inside an Alya project.".to_string(),
            );
        }
    };
    let manifest_path = manifest_dir.join("alya.toml");
    let manifest_src = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let mut manifest = parse_manifest(&manifest_src)?;

    if manifest.dependencies.is_empty() {
        println!("No dependencies declared in alya.toml.");
        return Ok(());
    }

    println!("Checking dependencies for updates in alya.toml...\n");

    let lock_path = if manifest_dir.join("alya.lock").exists() {
        manifest_dir.join("alya.lock")
    } else {
        manifest_dir.join("Alya.lock")
    };
    let mut lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };

    let mut rows: Vec<UpdateRow> = Vec::new();
    let mut upgradable_count = 0usize;

    for (name, dep) in &manifest.dependencies {
        match dep {
            DependencySource::Version(cur_ver) => {
                let url = resolve_registry_url(name);
                let tags = query_remote_tags(&url);
                if let Some(latest_tag) = find_latest_semver_tag(&tags) {
                    let latest_clean = latest_tag.trim_start_matches(['v', 'V']);
                    let cur_clean = cur_ver.trim_start_matches(['v', 'V']);
                    if compare_semver(latest_clean, cur_clean) == std::cmp::Ordering::Greater {
                        upgradable_count += 1;
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: cur_ver.clone(),
                            latest: latest_clean.to_string(),
                            status: format!("Update available ({} -> {})", cur_ver, latest_clean),
                            can_upgrade: true,
                            new_source: Some(DependencySource::Version(latest_clean.to_string())),
                            clear_cache_key: None,
                        });
                    } else {
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: cur_ver.clone(),
                            latest: latest_clean.to_string(),
                            status: "Up to date".to_string(),
                            can_upgrade: false,
                            new_source: None,
                            clear_cache_key: None,
                        });
                    }
                } else {
                    rows.push(UpdateRow {
                        name: name.clone(),
                        current: cur_ver.clone(),
                        latest: cur_ver.clone(),
                        status: "Up to date (no remote tags)".to_string(),
                        can_upgrade: false,
                        new_source: None,
                        clear_cache_key: None,
                    });
                }
            }
            DependencySource::Git {
                url,
                tag,
                branch,
                rev,
            } => {
                if let Some(cur_tag) = tag {
                    let tags = query_remote_tags(url);
                    if let Some(latest_tag) = find_latest_semver_tag(&tags) {
                        let latest_clean = latest_tag.trim_start_matches(['v', 'V']);
                        let cur_clean = cur_tag.trim_start_matches(['v', 'V']);
                        if compare_semver(latest_clean, cur_clean) == std::cmp::Ordering::Greater {
                            upgradable_count += 1;
                            let new_tag_str = if cur_tag.starts_with('v') {
                                format!("v{}", latest_clean)
                            } else {
                                latest_clean.to_string()
                            };
                            rows.push(UpdateRow {
                                name: name.clone(),
                                current: cur_tag.clone(),
                                latest: new_tag_str.clone(),
                                status: format!(
                                    "Update available ({} -> {})",
                                    cur_tag, new_tag_str
                                ),
                                can_upgrade: true,
                                new_source: Some(DependencySource::Git {
                                    url: url.clone(),
                                    tag: Some(new_tag_str),
                                    branch: None,
                                    rev: None,
                                }),
                                clear_cache_key: None,
                            });
                        } else {
                            // Same tag name on both sides: the tag itself may
                            // still have moved (re-pointed baselines). Compare
                            // resolved revisions to detect that.
                            let remote_rev = query_tag_rev(url, cur_tag);
                            let current_rev = current_dep_rev(&manifest_dir, &lock, name);
                            let short = |s: &str| s[..7.min(s.len())].to_string();
                            match (remote_rev, current_rev) {
                                (Some(rr), Some(cr)) if rr != cr => {
                                    upgradable_count += 1;
                                    let old_key =
                                        compute_cache_key_rev(name, cur_tag, url, Some(&cr));
                                    rows.push(UpdateRow {
                                        name: name.clone(),
                                        current: format!("{} ({})", cur_tag, short(&cr)),
                                        latest: format!("{} ({})", cur_tag, short(&rr)),
                                        status: format!(
                                            "Update available (tag {} moved: {} -> {})",
                                            cur_tag,
                                            short(&cr),
                                            short(&rr)
                                        ),
                                        can_upgrade: true,
                                        new_source: None,
                                        clear_cache_key: Some(old_key),
                                    });
                                }
                                _ => {
                                    rows.push(UpdateRow {
                                        name: name.clone(),
                                        current: cur_tag.clone(),
                                        latest: cur_tag.clone(),
                                        status: "Up to date".to_string(),
                                        can_upgrade: false,
                                        new_source: None,
                                        clear_cache_key: None,
                                    });
                                }
                            }
                        }
                    } else {
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: cur_tag.clone(),
                            latest: cur_tag.clone(),
                            status: "Up to date (no remote tags)".to_string(),
                            can_upgrade: false,
                            new_source: None,
                            clear_cache_key: None,
                        });
                    }
                } else if let Some(b) = branch {
                    let remote_head = query_remote_branch_head(url, b);
                    let short_remote_sha = remote_head.as_deref().map(|s| &s[..7.min(s.len())]);
                    let latest_disp = if let Some(sha) = short_remote_sha {
                        format!("{} ({})", b, sha)
                    } else {
                        b.clone()
                    };

                    let local_pkg = manifest_dir.join(".alya").join("packages").join(name);
                    let cache_key = compute_cache_key(name, b, url);
                    let cached_dir = get_global_cache_dir().map(|c| c.join(&cache_key));

                    let locked_rev = lock
                        .as_ref()
                        .and_then(|l| l.packages.iter().find(|p| &p.name == name))
                        .and_then(|p| parse_git_source_rev(&p.source));

                    let local_rev = fs::read_to_string(local_pkg.join(".alya-rev"))
                        .ok()
                        .or_else(|| {
                            cached_dir
                                .as_ref()
                                .and_then(|c| fs::read_to_string(c.join(".alya-rev")).ok())
                        })
                        .map(|s| s.trim().to_string())
                        .or(locked_rev);

                    let short_local_sha = local_rev.as_deref().map(|s| &s[..7.min(s.len())]);
                    let current_disp = if let Some(sha) = short_local_sha {
                        format!("{} ({})", b, sha)
                    } else {
                        format!("branch '{}'", b)
                    };

                    let is_already_at_head = match (&local_rev, &remote_head) {
                        (Some(l), Some(r)) => l == r,
                        _ => false,
                    };

                    if is_already_at_head {
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: current_disp,
                            latest: latest_disp,
                            status: "Up to date".to_string(),
                            can_upgrade: false,
                            new_source: None,
                            clear_cache_key: None,
                        });
                    } else {
                        upgradable_count += 1;
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: current_disp,
                            latest: latest_disp,
                            status: if upgrade {
                                "Branch refreshed to latest commit".to_string()
                            } else {
                                "New commit available on branch".to_string()
                            },
                            can_upgrade: true,
                            new_source: None,
                            clear_cache_key: Some(cache_key),
                        });
                    }
                } else if let Some(r) = rev {
                    let short_rev = &r[..7.min(r.len())];
                    rows.push(UpdateRow {
                        name: name.clone(),
                        current: format!("rev {}", short_rev),
                        latest: format!("rev {}", short_rev),
                        status: "Pinned commit (immutable)".to_string(),
                        can_upgrade: false,
                        new_source: None,
                        clear_cache_key: None,
                    });
                } else {
                    rows.push(UpdateRow {
                        name: name.clone(),
                        current: "git".to_string(),
                        latest: "git".to_string(),
                        status: "Up to date".to_string(),
                        can_upgrade: false,
                        new_source: None,
                        clear_cache_key: None,
                    });
                }
            }
            DependencySource::Path { path } => {
                rows.push(UpdateRow {
                    name: name.clone(),
                    current: path.clone(),
                    latest: path.clone(),
                    status: "Local path (pinned)".to_string(),
                    can_upgrade: false,
                    new_source: None,
                    clear_cache_key: None,
                });
            }
        }
    }

    println!(
        "  {:<16} {:<18} {:<18} {:<34}",
        "PACKAGE", "CURRENT", "LATEST", "STATUS"
    );
    println!("  {}", "-".repeat(88));
    for r in &rows {
        let cur_disp = if r.can_upgrade && r.current != r.latest {
            format!("{:<15} →", r.current)
        } else {
            format!("{:<17}", r.current)
        };
        println!(
            "  {:<16} {:<18} {:<18} {:<34}",
            r.name, cur_disp, r.latest, r.status
        );
    }

    if !upgrade {
        if upgradable_count > 0 {
            println!(
                "\n{} package(s) can be upgraded or refreshed.",
                upgradable_count
            );
            println!("Run 'alya update -u' (or 'alya update --upgrade') to upgrade alya.toml and re-lock.");
        } else {
            println!("\nAll dependencies are up to date!");
        }
        return Ok(());
    }

    if upgradable_count == 0 {
        println!("\nAll dependencies are already up to date!");
        return Ok(());
    }

    let mut toml_changed = false;
    for r in &rows {
        if let Some(ref new_src) = r.new_source {
            manifest
                .dependencies
                .insert(r.name.clone(), new_src.clone());
            toml_changed = true;
        }
        if let Some(ref cache_key) = r.clear_cache_key {
            if let Some(global_cache) = get_global_cache_dir() {
                let cached_dir = global_cache.join(cache_key);
                if cached_dir.exists() {
                    let _ = fs::remove_dir_all(&cached_dir);
                }
            }
            let local_pkg = manifest_dir.join(".alya").join("packages").join(&r.name);
            if local_pkg.exists() {
                let _ = fs::remove_dir_all(&local_pkg);
            }
            if let Some(ref mut l) = lock {
                l.packages.retain(|p| p.name != r.name);
                let _ = fs::write(&lock_path, serialize_lockfile(l));
            }
        }
    }

    if toml_changed {
        fs::write(&manifest_path, serialize_manifest(&manifest))
            .map_err(|e| format!("Failed to update alya.toml: {}", e))?;
        println!("\n✓ Upgraded dependencies in alya.toml.");
    } else {
        println!("\nNo version changes needed in alya.toml.");
    }

    println!("Resolving and locking updated dependencies...\n");
    run_install()?;
    println!("\n✓ All dependencies updated successfully!");
    Ok(())
}

pub fn print_pkg_help() {
    println!("Alya Package Manager (alya pkg)");
    println!("Manage project dependencies, manifests (alya.toml), and lockfiles (alya.lock).\n");
    println!("USAGE:");
    println!("  alya pkg <COMMAND> [OPTIONS]");
    println!("  alya init [path] [OPTIONS]          # Shortcut for pkg init");
    println!("  alya add <name> [OPTIONS]           # Shortcut for pkg add");
    println!("  alya install                        # Shortcut for pkg install");
    println!("  alya update [-u | --upgrade]        # Shortcut for pkg update");
    println!("  alya outdated                       # Shortcut for pkg outdated");
    println!("  alya cache                          # Shortcut for pkg cache");
    println!("  alya clean                          # Shortcut for pkg clean\n");
    println!("COMMANDS:");
    println!("  init [path]        Initialize a new Alya package in [path] (default: .)");
    println!("  add <name>         Add a new dependency to alya.toml");
    println!("  install            Resolve and lock all dependencies specified in alya.toml");
    println!("  list               List project dependencies and lock integrity status");
    println!(
        "  update [-u]        Check or upgrade dependencies (-u rewrites alya.toml & re-locks)"
    );
    println!("  outdated           Check for newer versions of dependencies without upgrading");
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
    println!("  --version <ver>    Specify semantic version constraint\n");
    println!("OPTIONS FOR 'update':");
    println!(
        "  -u, --upgrade      Rewrite alya.toml with latest versions and re-lock dependencies\n"
    );
    println!("EXAMPLES:");
    println!("  alya init my_app");
    println!("  alya add http                       # Add official package via short-name");
    println!("  alya install                        # Install & lock dependencies");
    println!("  alya update                         # Check for newer package versions");
    println!("  alya update -u                      # Upgrade alya.toml and re-lock");
    println!("  alya pkg outdated                   # Check outdated packages (read-only)");
    println!("  alya pkg cache");
    println!("  alya pkg clean");
}
