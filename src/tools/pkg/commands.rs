use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use super::cache::{get_global_cache_dir, run_cache, run_clean};
use super::discovery::{find_manifest_dir, find_package_entry};
use super::features::{
    close_features, dep_feature_requests, enabled_dependencies, resolve_active_features,
};
use super::hash::{
    compute_cache_key, compute_cache_key_rev, compute_package_checksum, verify_package_checksum,
    ChecksumVerdict,
};
use super::lock::{format_git_source, parse_git_source_rev, parse_lockfile, serialize_lockfile};
use super::manifest::{check_compiler_compatibility, parse_manifest, serialize_manifest};
use super::resolver::{
    coalesce_semver_versions, compare_semver, copy_dir_all, fetch_git_or_archive_dependency,
    query_remote_branch_head, query_remote_tags, query_tag_rev, resolve_package_spec,
    resolve_registry_url, semver_major,
};
use super::types::{
    DependencyEdge, DependencySource, LockedPackage, PackageInfo, PackageLock, PackageManifest,
    PkgCommand,
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
            optional,
        } => run_add(
            name,
            path.as_deref(),
            git.as_deref(),
            tag.as_deref(),
            branch.as_deref(),
            version.as_deref(),
            *optional,
        ),
        PkgCommand::Install {
            strict,
            features,
            no_default_features,
            packages,
            workspace,
            exclude,
        } => run_install(
            *strict,
            features,
            *no_default_features,
            packages,
            *workspace,
            exclude,
        ),
        PkgCommand::List => run_list(),
        PkgCommand::Update {
            upgrade,
            packages,
            workspace,
            exclude,
        } => run_update(*upgrade, packages, *workspace, exclude),
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
        features: BTreeMap::new(),
        profiles: BTreeMap::new(),
        workspace: None,
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
    optional: bool,
) -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    let manifest_path = manifest_dir.join("alya.toml");

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let mut manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;
    if super::manifest::is_virtual_workspace_root(&manifest) {
        return Err(
            "Error: cannot 'add' a dependency to a virtual workspace root (it declares members only). Run 'add' inside a member directory instead."
                .to_string(),
        );
    }

    let (resolved_name, auto_git) = resolve_package_spec(name);

    let (source, display_ver) = if let Some(p) = path {
        (
            DependencySource::Path {
                path: p.to_string(),
                edge: DependencyEdge {
                    optional,
                    ..DependencyEdge::plain()
                },
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
                edge: DependencyEdge {
                    optional,
                    ..DependencyEdge::plain()
                },
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
                edge: DependencyEdge {
                    optional,
                    ..DependencyEdge::plain()
                },
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
                        false,
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
        (
            DependencySource::Version {
                version: ver,
                edge: DependencyEdge {
                    optional,
                    ..DependencyEdge::plain()
                },
            },
            Some(v_disp),
        )
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

    run_install_in(&manifest_dir, false, &[], false, &[], false, &[])?;
    Ok(())
}

pub fn run_install(
    strict: bool,
    features: &[String],
    no_default_features: bool,
    packages: &[String],
    workspace: bool,
    exclude: &[String],
) -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    run_install_in(
        &manifest_dir,
        strict,
        features,
        no_default_features,
        packages,
        workspace,
        exclude,
    )
}

/// Lexically normalizes a root-relative display path (`a/b/../c` →
/// `a/c`, forward slashes). Pure component pass, no filesystem access, so
/// lock entries stay deterministic on every platform.
fn normalize_rel_display(path: &Path) -> String {
    use std::path::Component::*;
    let mut parts: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            CurDir => {}
            ParentDir => {
                parts.pop();
            }
            Normal(s) => parts.push(s.to_string_lossy().replace('\\', "/")),
            RootDir | Prefix(_) => {
                parts.push(comp.as_os_str().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    parts.join("/")
}

fn get_dep_major(dep: &DependencySource, from_dir: &Path) -> Option<u64> {
    match dep {
        DependencySource::Version { version: v, .. } => semver_major(v),
        DependencySource::Git { tag: Some(t), .. } => semver_major(t),
        DependencySource::Path { path, .. } => {
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

/// Locked registry version still satisfying `req`, if any. Cargo rule:
/// a locked version stays pinned even after being yanked; only fresh
/// selections skip yanked entries. Matches `name` and multi-major
/// `name-vN` lock entries.
pub(crate) fn locked_version_for_req(
    lock: Option<&PackageLock>,
    name: &str,
    req: &str,
) -> Option<String> {
    let prefix = format!("{name}-v");
    lock?.packages.iter().find_map(|p| {
        if p.name != name && !p.name.starts_with(&prefix) {
            return None;
        }
        super::index::version_satisfies_req(req, &p.version)
            .ok()
            .filter(|ok| *ok)
            .map(|_| p.version.clone())
    })
}

/// Yank warnings for a registry resolution. Never silent on either side:
/// honoring a yanked lock/pin says so, and silently switching to an older
/// version says what was skipped. Deduped per package via `reported`.
fn warn_on_yanked_resolution(
    name: &str,
    req: &str,
    locked_ver: Option<&str>,
    index_pick: Option<&super::index::IndexPick>,
    reported: &mut HashSet<String>,
) {
    let Some(pkg) = super::index::fetch_package_index(name) else {
        return;
    };
    let using = locked_ver.or_else(|| index_pick.map(|p| p.version.as_str()));
    match using {
        Some(u) if super::index::lookup_index_version(&pkg, u).is_some_and(|e| e.yanked) => {
            if reported.insert(format!("{name}:yanked-honored")) {
                println!(
                    "  Warning: '{}' {} is marked yanked in the package index; honoring as-is.",
                    name, u
                );
            }
        }
        Some(u) => {
            let skipped = super::index::select_index_version_including_yanked(&pkg, req)
                .ok()
                .flatten()
                .filter(|best| best.yanked && best.version != u);
            if let Some(best) = skipped {
                if reported.insert(format!("{name}:yanked-skipped")) {
                    println!(
                        "  Warning: '{}' newest match {} is yanked; installing {} instead.",
                        name, best.version, u
                    );
                }
            }
        }
        None => {
            // No lock, no index pick: an exact yanked pin falls through to
            // the git tag (installed as-is), anything else falls back too.
            if let Some(pinned) = super::index::lookup_index_version(&pkg, req) {
                if pinned.yanked && reported.insert(format!("{name}:yanked-pin")) {
                    println!(
                        "  Warning: '{}' {} is marked yanked in the package index; installing from git tag anyway.",
                        name, pinned.version
                    );
                }
            } else if let Some(best) =
                super::index::select_index_version_including_yanked(&pkg, req)
                    .ok()
                    .flatten()
                    .filter(|b| b.yanked)
            {
                if reported.insert(format!("{name}:yanked-only")) {
                    println!(
                        "  Warning: '{}' has no non-yanked match for '{}' (newest is yanked {}); falling back to git.",
                        name, req, best.version
                    );
                }
            }
        }
    }
}

fn ensure_dep_cached(
    name: &str,
    dep: &DependencySource,
    from_manifest_dir: &Path,
    existing_lock: Option<&PackageLock>,
    reported: &mut HashSet<String>,
    strict: bool,
) -> Result<PathBuf, String> {
    match dep {
        DependencySource::Path { path, .. } => {
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
            ..
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
                let mut fetch_res = fetch_git_or_archive_dependency(
                    name,
                    url,
                    tag.as_deref(),
                    branch.as_deref(),
                    effective_rev.as_deref(),
                    &cached_pkg_dir,
                    strict,
                );
                if fetch_res.is_err()
                    && !strict
                    && tag.is_some()
                    && branch.is_none()
                    && effective_rev.is_none()
                {
                    fetch_res = fetch_git_or_archive_dependency(
                        name,
                        url,
                        None,
                        None,
                        None,
                        &cached_pkg_dir,
                        strict,
                    );
                }
                fetch_res?;
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
        DependencySource::Version { version: v, .. } => {
            let url = resolve_registry_url(name);
            // Static index fast path: precise max-satisfying tag without
            // `git ls-remote`. Silent miss (no index, no match) keeps the
            // legacy derivation below.
            // A locked version stays pinned even when yanked (Cargo rule);
            // only fresh selections skip yanked entries.
            let locked_ver = locked_version_for_req(existing_lock, name, v);
            let index_pick = match &locked_ver {
                Some(_) => None,
                None => super::index::select_version(name, v)?,
            };
            warn_on_yanked_resolution(
                name,
                v,
                locked_ver.as_deref(),
                index_pick.as_ref(),
                reported,
            );
            let tag_cand: Option<String> = match (&locked_ver, &index_pick) {
                (Some(lv), _) => Some(if lv.starts_with('v') || lv.starts_with('V') {
                    lv.clone()
                } else {
                    format!("v{lv}")
                }),
                (None, Some(pick)) => Some(pick.tag.clone()),
                (None, None) => {
                    if v != "*" && !v.is_empty() {
                        Some(if v.starts_with('v') || v.starts_with('V') {
                            v.clone()
                        } else {
                            format!("v{}", v)
                        })
                    } else {
                        None
                    }
                }
            };
            let tag_or_branch = tag_cand.as_deref().unwrap_or("head");
            let source = format!("registry+{}#{}", url, tag_or_branch);

            let cache_dir = get_global_cache_dir()
                .unwrap_or_else(|| from_manifest_dir.join(".alya").join("cache"));
            let cache_key = compute_cache_key(name, tag_or_branch, &url);
            let cached_pkg_dir = cache_dir.join(&cache_key);

            let local_pkg_dir = from_manifest_dir.join(".alya").join("packages").join(name);
            let local_hit = if local_pkg_dir.exists() && local_pkg_dir.join("alya.toml").exists() {
                if let Ok(manifest_src) = fs::read_to_string(local_pkg_dir.join("alya.toml")) {
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

            let cache_hit =
                !local_hit && cached_pkg_dir.exists() && cached_pkg_dir.join("alya.toml").exists();

            let head_cache_key = compute_cache_key(name, "head", &url);
            let head_cached_dir = cache_dir.join(&head_cache_key);
            let head_hit = if !cache_hit
                && !local_hit
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

            if local_hit {
                let _ = copy_dir_all(&local_pkg_dir, &cached_pkg_dir, true);
                let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
                if reported.insert(format!("{}:{}", name, v)) {
                    println!(
                        "  Using existing package '{}' ({}) from .alya/packages",
                        name, v
                    );
                }
            } else if cache_hit {
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
                // Explicit-tarball index entries install with strict inline
                // checksum verification; anything else falls through.
                if let Some(pick) = &index_pick {
                    if let (Some(tar), Some(hex)) =
                        (pick.tarball.as_ref(), pick.checksum_hex.as_ref())
                    {
                        if super::index::install_index_tarball(
                            name,
                            tar,
                            hex,
                            &pick.tag,
                            &cached_pkg_dir,
                        )? {
                            let _ = fs::write(cached_pkg_dir.join(".alya-source"), &source);
                            return Ok(cached_pkg_dir);
                        }
                    }
                }
                let mut fetch_res = fetch_git_or_archive_dependency(
                    name,
                    &url,
                    tag_cand.as_deref(),
                    None,
                    None,
                    &cached_pkg_dir,
                    strict,
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
                            strict,
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
                    if fetch_res.is_err()
                        && local_pkg_dir.exists()
                        && local_pkg_dir.join("alya.toml").exists()
                    {
                        if let Ok(manifest_src) =
                            fs::read_to_string(local_pkg_dir.join("alya.toml"))
                        {
                            if let Ok(parsed) = parse_manifest(&manifest_src) {
                                if parsed.package.version == *v || v == "*" {
                                    let _ = copy_dir_all(&local_pkg_dir, &cached_pkg_dir, true);
                                    fetch_res = Ok(());
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

pub fn run_install_in(
    manifest_dir: &Path,
    strict: bool,
    features: &[String],
    no_default_features: bool,
    packages: &[String],
    workspace: bool,
    exclude: &[String],
) -> Result<(), String> {
    // Inside a workspace the lockfile and `.alya/packages` are shared at
    // the root and install always covers every member (a partial install
    // would corrupt the shared lock).
    if let Some(root) = super::workspace::find_workspace_root_from(manifest_dir) {
        if !packages.is_empty() || !exclude.is_empty() {
            return Err(
                "Error: '--package'/'--exclude' are not supported for 'install' in a workspace (install always covers all members)"
                    .to_string(),
            );
        }
        let _ = workspace;
        return run_install_workspace(&root, strict, features, no_default_features);
    }
    run_install_single(manifest_dir, strict, features, no_default_features)
}

/// Installs every workspace member into the shared root `.alya/packages`
/// with one root `alya.lock`. Member `--features` apply to each member.
fn run_install_workspace(
    root: &Path,
    strict: bool,
    features: &[String],
    no_default_features: bool,
) -> Result<(), String> {
    use super::workspace::workspace_targets;
    let members = workspace_targets(root)?;
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
    println!(
        "Workspace '{}': installing {} member{} ({})",
        root.display(),
        members.len(),
        if members.len() == 1 { "" } else { "s" },
        names.join(", ")
    );
    // Gather per-member (manifest, dir, closed active set) entries; the
    // shared stage below unifies features across the graph, then coalesces,
    // segregates, and locks everything together.
    let mut entries: Vec<(PackageManifest, PathBuf, BTreeSet<String>)> = Vec::new();
    for member in &members {
        let content = fs::read_to_string(member.dir.join("alya.toml"))
            .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
        let manifest = parse_manifest(&content)?;
        check_compiler_compatibility(&manifest)?;
        let active = resolve_active_features(&manifest, features, no_default_features)
            .map_err(|e| format!("Member '{}': {}", member.name, e))?;
        let enabled = enabled_dependencies(&manifest, &active);
        for name in manifest.dependencies.keys() {
            if !enabled.contains(name) {
                println!(
                    "  Skipping optional dependency '{}' of member '{}' (no active feature enables it)",
                    name, member.name
                );
            }
        }
        entries.push((manifest, member.dir.clone(), active));
    }
    if !features.is_empty() || no_default_features {
        let mut names: Vec<&str> = features.iter().map(|s| s.as_str()).collect();
        names.sort();
        println!(
            "Active features (per member): {}",
            if names.is_empty() {
                "(none)".to_string()
            } else {
                names.join(", ")
            }
        );
    }
    install_resolved(root, entries, strict)
}

/// Legacy single-package install: lockfile and packages beside the package.
fn run_install_single(
    manifest_dir: &Path,
    strict: bool,
    features: &[String],
    no_default_features: bool,
) -> Result<(), String> {
    let manifest_path = manifest_dir.join("alya.toml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let manifest = parse_manifest(&content)?;
    check_compiler_compatibility(&manifest)?;
    let active = resolve_active_features(&manifest, features, no_default_features)?;
    let enabled = enabled_dependencies(&manifest, &active);
    if !features.is_empty() || no_default_features {
        let mut names: Vec<&str> = active.iter().map(|s| s.as_str()).collect();
        names.sort();
        println!(
            "Active features: {}",
            if names.is_empty() {
                "(none)".to_string()
            } else {
                names.join(", ")
            }
        );
    }

    for name in manifest.dependencies.keys() {
        if !enabled.contains(name) {
            println!(
                "  Skipping optional dependency '{}' (no active feature enables it)",
                name
            );
        }
    }
    install_resolved(
        manifest_dir,
        vec![(manifest, manifest_dir.to_path_buf(), active)],
        strict,
    )
}

/// One node of the install-time feature graph: an entry (workspace
/// member or single package root, never installed) or a resolved package.
struct InstallNode {
    /// Entry roots have no install source; packages carry the coalesced one.
    source: Option<DependencySource>,
    /// First requester's dir (package fetch base) / entry member dir.
    base_dir: PathBuf,
    /// Packages without (or with unreadable) manifests still install;
    /// they just contribute no further edges.
    manifest: Option<PackageManifest>,
    /// Canonicalized id: `entry:{i}` or `pkg:{name}:{major:?}`.
    pkg_name: Option<String>,
    pkg_major: Option<u64>,
    seeds: BTreeSet<String>,
    closed: BTreeSet<String>,
    propagated: bool,
    is_entry: bool,
    /// (parent id, edge keeps defaults, edge features) per active edge.
    incoming: Vec<(String, bool, Vec<String>)>,
}

/// Shared install stages: graph discovery + feature-unification fixpoint +
/// SemVer coalescing, then major segregation and installation under
/// `lock_dir/.alya/packages` with the lockfile at `lock_dir/alya.lock`.
fn install_resolved(
    lock_dir: &Path,
    entries: Vec<(PackageManifest, PathBuf, BTreeSet<String>)>,
    strict: bool,
) -> Result<(), String> {
    let packages_dir = lock_dir.join(".alya").join("packages");
    let lock_path = if lock_dir.join("alya.lock").exists() {
        lock_dir.join("alya.lock")
    } else {
        lock_dir.join("Alya.lock")
    };
    let existing_lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };

    // Stage 1: Dependency Graph Discovery + feature-unification fixpoint
    // + SemVer Coalescing. Seeds and incoming edges only grow, and nodes
    // propagate only on growth, so the worklist terminates.
    let mut nodes: BTreeMap<String, InstallNode> = BTreeMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut reported: HashSet<String> = HashSet::new();
    for (i, (manifest, dir, active)) in entries.into_iter().enumerate() {
        let id = format!("entry:{i}");
        nodes.insert(
            id.clone(),
            InstallNode {
                source: None,
                base_dir: dir,
                manifest: Some(manifest),
                pkg_name: None,
                pkg_major: None,
                seeds: active,
                closed: BTreeSet::new(),
                propagated: false,
                is_entry: true,
                incoming: Vec::new(),
            },
        );
        queue.push_back(id);
    }

    while let Some(id) = queue.pop_front() {
        // Closed set under the currently recorded incoming edges, plus the
        // outgoing requests (dep source + feats) for every enabled dep.
        type InstallRequests = Vec<(String, DependencySource, Vec<String>)>;
        let (closed_new, base_dir, requests): (BTreeSet<String>, PathBuf, InstallRequests) = {
            let node = match nodes.get(&id) {
                Some(n) => n,
                None => continue,
            };
            let Some(manifest) = node.manifest.as_ref() else {
                continue;
            };
            let mut seeds = node.seeds.clone();
            if !node.is_entry
                && manifest.features.contains_key("default")
                && (node.incoming.is_empty() || node.incoming.iter().any(|(_, keep, _)| *keep))
            {
                seeds.insert("default".to_string());
            }
            let closed = close_features(manifest, &seeds);
            let enabled = enabled_dependencies(manifest, &closed);
            let mut slash: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (dep, feat) in dep_feature_requests(manifest, &closed) {
                slash.entry(dep).or_default().push(feat);
            }
            let mut reqs = Vec::new();
            for dep_name in &enabled {
                if let Some(dep) = manifest.dependencies.get(dep_name) {
                    let mut feats = slash.remove(dep_name).unwrap_or_default();
                    feats.extend(dep.edge().features.iter().cloned());
                    reqs.push((dep_name.clone(), dep.clone(), feats));
                }
            }
            (closed, node.base_dir.clone(), reqs)
        };
        let should_propagate = match nodes.get(&id) {
            Some(n) => !n.propagated || closed_new != n.closed,
            None => false,
        };
        if should_propagate {
            if let Some(n) = nodes.get_mut(&id) {
                n.closed = closed_new;
                n.propagated = true;
            }
        } else {
            continue;
        }
        for (dep_name, dep_source, feats) in requests {
            let maj = get_dep_major(&dep_source, &base_dir);
            let child_id = format!("pkg:{}:{maj:?}", dep_name);
            if !nodes.contains_key(&child_id) {
                let source_dir = ensure_dep_cached(
                    &dep_name,
                    &dep_source,
                    &base_dir,
                    existing_lock.as_ref(),
                    &mut reported,
                    strict,
                )?;
                let manifest = fs::read_to_string(source_dir.join("alya.toml"))
                    .ok()
                    .and_then(|c| parse_manifest(&c).ok());
                nodes.insert(
                    child_id.clone(),
                    InstallNode {
                        source: Some(dep_source.clone()),
                        base_dir: source_dir,
                        manifest,
                        pkg_name: Some(dep_name.clone()),
                        pkg_major: maj,
                        seeds: BTreeSet::new(),
                        closed: BTreeSet::new(),
                        propagated: false,
                        is_entry: false,
                        incoming: Vec::new(),
                    },
                );
            } else if let Some(node) = nodes.get_mut(&child_id) {
                // SemVer coalescing on repeat requests (install source
                // only; feature edges merge below regardless).
                if let (
                    Some(DependencySource::Version { version: cur, .. }),
                    DependencySource::Version { version: v2, .. },
                ) = (node.source.as_mut(), &dep_source)
                {
                    let merged = coalesce_semver_versions(cur, v2)?;
                    if merged != cur.as_str() {
                        *cur = merged.to_string();
                    }
                }
            }
            let keep = dep_source.edge().default_features;
            let edge_feats = dep_source.edge().features.clone();
            // Strict target validation: every requested feature must exist
            // in the child's manifest (feature or optional dependency).
            // Typos fail here with a named edge; the compiler stays lenient
            // (uninstalled leaves fall back to defaults).
            if let Some(child) = nodes.get(&child_id) {
                if let Some(child_manifest) = child.manifest.as_ref() {
                    let parent_name = nodes
                        .get(&id)
                        .and_then(|n| n.manifest.as_ref())
                        .map(|m| m.package.name.clone())
                        .unwrap_or_else(|| id.clone());
                    for feat in &feats {
                        if !child_manifest.features.contains_key(feat)
                            && !child_manifest.dependencies.contains_key(feat)
                        {
                            return Err(format!(
                                "Package '{}' requests unknown feature '{}' on dependency '{}' (no such feature or optional dependency in '{}'; check `dep/feat` members and edge `features`)",
                                parent_name, feat, dep_name,
                                child_manifest.package.name,
                            ));
                        }
                    }
                }
            }
            let mut child_grew = false;
            if let Some(child) = nodes.get_mut(&child_id) {
                if !child.incoming.iter().any(|(p, _, _)| p == &id) {
                    child.incoming.push((id.clone(), keep, edge_feats));
                    child_grew = true;
                }
                for feat in feats {
                    if child.seeds.insert(feat) {
                        child_grew = true;
                    }
                }
            }
            if child_grew && !queue.contains(&child_id) {
                queue.push_back(child_id);
            }
        }
    }

    // Legacy shape for stages 2-3: one install source + owner per package.
    let mut resolved_requests: BTreeMap<(String, Option<u64>), (DependencySource, PathBuf)> =
        BTreeMap::new();
    for node in nodes.values() {
        if let (Some(name), Some(source)) = (node.pkg_name.clone(), node.source.clone()) {
            resolved_requests.insert((name, node.pkg_major), (source, node.base_dir.clone()));
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
            strict,
        )?;
        let is_path_dep = matches!(dep, DependencySource::Path { .. });

        let (target_dir, actual_source_str) = if is_path_dep && !is_multi {
            let path_str = match &dep {
                DependencySource::Path { path, .. } => path.replace('\\', "/"),
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
            // A tree installed from the curated release asset is exempt: the
            // asset is already TLS + SHA-256 verified at fetch time, and it
            // intentionally omits dev-only trees (tests/, benches/), so its
            // content checksum legitimately differs from a source-tarball
            // install. The fresh checksum is still recorded below.
            let from_asset = dest_dir.join(".alya-asset").exists();
            if let Some(locked) = existing_lock.as_ref().and_then(|l| {
                l.packages
                    .iter()
                    .find(|p| p.name == folder_name)
                    .or_else(|| l.packages.iter().find(|p| p.name == *name))
            }) {
                if !locked.checksum.is_empty() && !from_asset {
                    let lock_version = existing_lock.as_ref().map(|l| l.version).unwrap_or(1);
                    match verify_package_checksum(&dest_dir, &locked.checksum, lock_version)? {
                        ChecksumVerdict::Match => {}
                        ChecksumVerdict::LegacyHealed => {
                            println!(
                                "  Notice: '{}' lock checksum healed to platform-independent digest (lockfile v1 -> v2).",
                                name
                            );
                        }
                        ChecksumVerdict::Mismatch => {
                            let actual = compute_package_checksum(&dest_dir)?;
                            return Err(format!(
                                "Checksum mismatch for '{}': installed content does not match alya.lock (locked {}, got {}). Delete the lock or reinstall to proceed.",
                                name, locked.checksum, actual
                            ));
                        }
                    }
                } else if from_asset {
                    println!(
                        "  Notice: '{}' installed from verified release asset; lock checksum refreshed (curated tree).",
                        name
                    );
                }
            }

            let source_str = match &dep {
                DependencySource::Version { version: v, .. } => {
                    let url = resolve_registry_url(&name);
                    let v_tag = if v.starts_with('v') || v.starts_with('V') {
                        v.clone()
                    } else {
                        format!("v{}", v)
                    };
                    format!("registry+{}#{}", url, v_tag)
                }
                DependencySource::Path { path, .. } => {
                    format!("path:{}", path.replace('\\', "/"))
                }
                DependencySource::Git {
                    url,
                    branch,
                    tag,
                    rev,
                    ..
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
        // Path deps resolve in place (`crates/app/../calc`): normalize the
        // `..` lexically so the shared lock records a clean root-relative
        // path (`crates/calc/...`). Pure lexical pass, no filesystem access.
        let rel_entry = normalize_rel_display(entry.strip_prefix(lock_dir).unwrap_or(&entry));
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
        // v2 checksums are line-ending normalized (platform-independent).
        // v1 locks keep verifying (legacy first, normalized fallback) and
        // are rewritten as v2 here, completing the migration on install.
        version: 2,
        packages: locked_packages,
    };

    let lockfile_path = lock_dir.join("alya.lock");
    fs::write(&lockfile_path, serialize_lockfile(&lock))
        .map_err(|e| format!("Failed to write alya.lock: {}", e))?;

    // On case-sensitive filesystems, remove legacy Alya.lock if distinct from alya.lock
    let legacy_lock = lock_dir.join("Alya.lock");
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

/// Workspace `list`: members with versions plus shared-lock status.
fn run_list_workspace(root: &Path) -> Result<(), String> {
    use super::workspace::workspace_targets;
    let members = workspace_targets(root)?;
    println!(
        "Workspace: {} ({} member{})",
        root.display(),
        members.len(),
        if members.len() == 1 { "" } else { "s" }
    );
    let lock_path = if root.join("alya.lock").exists() {
        root.join("alya.lock")
    } else {
        root.join("Alya.lock")
    };
    let lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };
    for member in &members {
        let content = fs::read_to_string(member.dir.join("alya.toml"))
            .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
        let manifest = parse_manifest(&content)?;
        let locked_note = match &lock {
            Some(_) => "locked",
            None => "not locked - run 'alya install'",
        };
        println!(
            "  • {:<16} v{:<10} [{}]",
            manifest.package.name, manifest.package.version, locked_note
        );
    }
    if let Some(l) = lock {
        println!(
            "\nShared lock: {} package{} in {}",
            l.packages.len(),
            if l.packages.len() == 1 { "" } else { "s" },
            lock_path.display()
        );
    }
    Ok(())
}

pub fn run_list() -> Result<(), String> {
    let manifest_dir = find_manifest_dir().ok_or_else(|| {
        "Error: Could not find 'alya.toml' in current directory or any parent.".to_string()
    })?;
    if let Some(root) = super::workspace::find_workspace_root_from(&manifest_dir) {
        return run_list_workspace(&root);
    }
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
            DependencySource::Path { path, .. } => {
                let opt = if dep.is_optional() { " (optional)" } else { "" };
                format!("path: {}{}", path, opt)
            }
            DependencySource::Git {
                url, tag, branch, ..
            } => {
                let mut s = format!("git: {}", url);
                if let Some(t) = tag {
                    s.push_str(&format!(" (tag: {})", t));
                } else if let Some(b) = branch {
                    s.push_str(&format!(" (branch: {})", b));
                }
                if dep.is_optional() {
                    s.push_str(" (optional)");
                }
                s
            }
            DependencySource::Version { version: v, .. } => {
                let opt = if dep.is_optional() { " (optional)" } else { "" };
                format!("version: {}{}", v, opt)
            }
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
                    let lock_version = lock.as_ref().map(|l| l.version).unwrap_or(1);
                    match verify_package_checksum(&installed_dir, &lp.checksum, lock_version) {
                        Ok(ChecksumVerdict::Match) | Ok(ChecksumVerdict::LegacyHealed) => {
                            "integrity: ok"
                        }
                        Ok(ChecksumVerdict::Mismatch) => "integrity: MISMATCH",
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

/// Max semver git tag excluding index-yanked versions. Returns the tag,
/// whether the index contributed, and is the shared LATEST rule for
/// `update`: yanked tags are never proposed. A missing/unreachable index
/// (or no entry) keeps the pure-git result — offline flows never hard-fail.
pub(crate) fn latest_tag_excluding_yanked(name: &str, tags: &[String]) -> (Option<String>, bool) {
    let git_latest = || super::resolver::find_latest_semver_tag(tags).map(|s| s.to_string());
    let Some(pkg) = super::index::fetch_package_index(name) else {
        return (git_latest(), false);
    };
    if pkg.versions.is_empty() {
        return (git_latest(), false);
    }
    let yanked: std::collections::HashSet<String> = pkg
        .versions
        .iter()
        .filter(|v| v.yanked)
        .map(|v| v.version.trim().trim_start_matches(['v', 'V']).to_string())
        .collect();
    let survivors: Vec<String> = tags
        .iter()
        .filter(|t| {
            let clean = t.trim().trim_start_matches(['v', 'V']);
            super::resolver::parse_semver(t).is_some() && !yanked.contains(clean)
        })
        .cloned()
        .collect();
    (
        super::resolver::find_latest_semver_tag(&survivors).map(|s| s.to_string()),
        true,
    )
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

pub fn run_update(
    upgrade: bool,
    packages: &[String],
    workspace: bool,
    exclude: &[String],
) -> Result<(), String> {
    let manifest_dir = match find_manifest_dir() {
        Some(d) => d,
        None => {
            return Err(
                "No 'alya.toml' found. Please run this command inside an Alya project.".to_string(),
            );
        }
    };
    // Inside a workspace the shared root lock covers every target, so
    // update always spans all of them (a partial update would corrupt the
    // shared lock) — same rule as `install`.
    if let Some(root) = super::workspace::find_workspace_root_from(&manifest_dir) {
        if !packages.is_empty() || !exclude.is_empty() {
            return Err(
                "Error: '--package'/'--exclude' are not supported for 'update' in a workspace (update always covers all members)"
                    .to_string(),
            );
        }
        let _ = workspace;
        return run_update_workspace(&root, upgrade);
    }
    let _ = (packages, workspace, exclude);
    run_update_single(&manifest_dir, upgrade)
}

/// Workspace update: checks every target, rewrites changed member
/// manifests, then re-locks once into the shared root lockfile.
fn run_update_workspace(root: &Path, upgrade: bool) -> Result<(), String> {
    use super::workspace::workspace_targets;
    let members = workspace_targets(root)?;
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
    println!(
        "Workspace '{}': checking {} member{} ({})",
        root.display(),
        members.len(),
        if members.len() == 1 { "" } else { "s" },
        names.join(", ")
    );
    let lock_path = if root.join("alya.lock").exists() {
        root.join("alya.lock")
    } else {
        root.join("Alya.lock")
    };
    let mut lock = if lock_path.exists() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|c| parse_lockfile(&c).ok())
    } else {
        None
    };
    let mut total = 0usize;
    for member in &members {
        if members.len() > 1 {
            println!("\n--- workspace member: {} ---", member.name);
        }
        total += update_one_manifest(&member.dir, root, &mut lock, &lock_path, upgrade)?;
    }
    if !upgrade {
        return Ok(());
    }
    if total == 0 {
        println!("\nAll workspace dependencies are already up to date!");
        return Ok(());
    }
    println!("Resolving and locking updated dependencies...\n");
    run_install_workspace(root, false, &[], false)?;
    println!("\n✓ All workspace dependencies updated successfully!");
    Ok(())
}

fn run_update_single(manifest_dir: &Path, upgrade: bool) -> Result<(), String> {
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
    let upgradable =
        update_one_manifest(manifest_dir, manifest_dir, &mut lock, &lock_path, upgrade)?;
    if !upgrade || upgradable == 0 {
        return Ok(());
    }
    println!("Resolving and locking updated dependencies...\n");
    run_install(false, &[], false, &[], false, &[])?;
    println!("\n✓ All dependencies updated successfully!");
    Ok(())
}

/// Checks one manifest for updates, prints its table, and — with `upgrade`
/// — rewrites changed pins, clears stale caches, and prunes the lock.
/// `member_dir` owns the `alya.toml`; `install_root` owns `.alya/packages`
/// and the lockfile (the workspace root in workspace mode). Returns the
/// upgradable count. Checkout-only reporting (`upgrade == false`) and the
/// all-up-to-date early-outs live here so single and workspace flows share
/// them exactly.
fn update_one_manifest(
    member_dir: &Path,
    install_root: &Path,
    lock: &mut Option<PackageLock>,
    lock_path: &Path,
    upgrade: bool,
) -> Result<usize, String> {
    let manifest_path = member_dir.join("alya.toml");
    let manifest_src = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    let mut manifest = parse_manifest(&manifest_src)?;

    if manifest.dependencies.is_empty() {
        println!("No dependencies declared in alya.toml.");
        return Ok(0);
    }

    println!("Checking dependencies for updates in alya.toml...\n");

    let (rows, upgradable_count, index_sourced) =
        collect_update_rows(&manifest, install_root, lock, upgrade);

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
    if index_sourced > 0 {
        println!(
            "  LATEST for {} package(s) resolved via package index (yank-filtered).",
            index_sourced
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
        return Ok(0);
    }

    if upgradable_count == 0 {
        println!("\nAll dependencies are already up to date!");
        return Ok(0);
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
            let local_pkg = install_root.join(".alya").join("packages").join(&r.name);
            if local_pkg.exists() {
                let _ = fs::remove_dir_all(&local_pkg);
            }
            if let Some(ref mut l) = lock {
                l.packages.retain(|p| p.name != r.name);
                let _ = fs::write(lock_path, serialize_lockfile(l));
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
    Ok(upgradable_count)
}

/// Builds the update table rows for one manifest. Pure remote inspection:
/// no files are written here.
fn collect_update_rows(
    manifest: &PackageManifest,
    install_root: &Path,
    lock: &Option<PackageLock>,
    upgrade: bool,
) -> (Vec<UpdateRow>, usize, usize) {
    let mut rows: Vec<UpdateRow> = Vec::new();
    let mut upgradable_count = 0usize;
    let mut index_sourced = 0usize;

    for (name, dep) in &manifest.dependencies {
        match dep {
            DependencySource::Version {
                version: cur_ver,
                edge,
            } => {
                let url = resolve_registry_url(name);
                let tags = query_remote_tags(&url);
                let (latest_opt, via_index) = latest_tag_excluding_yanked(name, &tags);
                if via_index {
                    index_sourced += 1;
                }
                if let Some(latest_tag) = latest_opt {
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
                            new_source: Some(DependencySource::Version {
                                version: latest_clean.to_string(),
                                edge: edge.clone(),
                            }),
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
                } else if tags.is_empty() {
                    rows.push(UpdateRow {
                        name: name.clone(),
                        current: cur_ver.clone(),
                        latest: cur_ver.clone(),
                        status: "Up to date (no remote tags)".to_string(),
                        can_upgrade: false,
                        new_source: None,
                        clear_cache_key: None,
                    });
                } else {
                    // Tags exist but every semver one is yanked in the index:
                    // propose nothing rather than a yanked LATEST.
                    rows.push(UpdateRow {
                        name: name.clone(),
                        current: cur_ver.clone(),
                        latest: cur_ver.clone(),
                        status: "Up to date (latest indexed versions yanked)".to_string(),
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
                edge,
            } => {
                if let Some(cur_tag) = tag {
                    let tags = query_remote_tags(url);
                    let (latest_opt, via_index) = latest_tag_excluding_yanked(name, &tags);
                    if via_index {
                        index_sourced += 1;
                    }
                    if let Some(latest_tag) = latest_opt {
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
                                    edge: edge.clone(),
                                }),
                                clear_cache_key: None,
                            });
                        } else {
                            // Same tag name on both sides: the tag itself may
                            // still have moved (re-pointed baselines). Compare
                            // resolved revisions to detect that.
                            let remote_rev = query_tag_rev(url, cur_tag);
                            let current_rev = current_dep_rev(install_root, lock, name);
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
                    } else if tags.is_empty() {
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: cur_tag.clone(),
                            latest: cur_tag.clone(),
                            status: "Up to date (no remote tags)".to_string(),
                            can_upgrade: false,
                            new_source: None,
                            clear_cache_key: None,
                        });
                    } else {
                        rows.push(UpdateRow {
                            name: name.clone(),
                            current: cur_tag.clone(),
                            latest: cur_tag.clone(),
                            status: "Up to date (latest indexed versions yanked)".to_string(),
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

                    let local_pkg = install_root.join(".alya").join("packages").join(name);
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
            DependencySource::Path { path, .. } => {
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

    (rows, upgradable_count, index_sourced)
}

pub fn print_pkg_help() {
    crate::cli::help::print_pkg_help();
}
