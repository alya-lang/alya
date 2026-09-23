use super::hash::sha256_hex;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

/// Curated per-release source archive published by the standard
/// `release.yml` workflow (`alya-pkg.tar.gz` + detached SHA-256).
/// Fixed names (no version embedded): the release tag in the download URL
/// already scopes them, so installers never face `v`-prefix ambiguity.
pub const RELEASE_ASSET_TARBALL: &str = "alya-pkg.tar.gz";
pub const RELEASE_ASSET_CHECKSUM: &str = "alya-pkg.tar.gz.sha256";

/// Splits a repository URL into `(owner, repo)` for `github.com` remotes
/// (`https://`, `http://`, `git@` forms, optional `.git` suffix).
/// Returns `None` for any other host.
fn parse_github_owner_repo(url: &str) -> Option<(String, String)> {
    let clean = url.trim_end_matches('/').trim_end_matches(".git");
    let gh_sub = clean
        .strip_prefix("https://github.com/")
        .or_else(|| clean.strip_prefix("http://github.com/"))
        .or_else(|| clean.strip_prefix("git@github.com:"));
    let sub = gh_sub?;
    let mut parts = sub.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// Curated release-asset URL pairs `(tarball, sha256)` for a pinned tag on
/// a GitHub-hosted package, including the `v`-prefix alternate. Empty for
/// non-GitHub hosts: assets are a GitHub Releases convention, and branch /
/// rev pins never have per-release assets by definition.
pub fn resolve_release_asset_urls(url: &str, tag: &str) -> Vec<(String, String)> {
    let Some((owner, repo)) = parse_github_owner_repo(url) else {
        return Vec::new();
    };
    let mut tags = vec![tag.to_string()];
    if !tag.starts_with('v') && !tag.starts_with('V') {
        tags.push(format!("v{}", tag));
    } else if let Some(stripped) = tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')) {
        tags.push(stripped.to_string());
    }
    tags.iter()
        .map(|t| {
            (
                format!(
                    "https://github.com/{}/{}/releases/download/{}/{}",
                    owner, repo, t, RELEASE_ASSET_TARBALL
                ),
                format!(
                    "https://github.com/{}/{}/releases/download/{}/{}",
                    owner, repo, t, RELEASE_ASSET_CHECKSUM
                ),
            )
        })
        .collect()
}

pub fn resolve_registry_url(name: &str) -> String {
    if let Ok(reg) = env::var("ALYA_REGISTRY") {
        let trimmed = reg.trim().trim_end_matches('/');
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("file://")
            || trimmed.starts_with("git@")
            || trimmed.starts_with("ssh://")
        {
            if trimmed.ends_with('/') || trimmed.ends_with('\\') {
                format!("{}{}.git", trimmed, name)
            } else {
                format!("{}/{}.git", trimmed, name)
            }
        } else if trimmed.contains('/') || trimmed.contains('\\') {
            format!("{}/{}", trimmed.replace('\\', "/"), name)
        } else {
            format!("https://github.com/{}/{}.git", trimmed, name)
        }
    } else {
        format!("https://github.com/alya-lang/{}.git", name)
    }
}

pub fn resolve_package_spec(spec: &str) -> (String, Option<String>) {
    let clean = spec.trim();
    if clean.starts_with("http://")
        || clean.starts_with("https://")
        || clean.starts_with("git@")
        || clean.starts_with("ssh://")
        || clean.starts_with("file://")
    {
        let repo_part = clean.split('/').next_back().unwrap_or(clean);
        let name = repo_part.trim_end_matches(".git");
        return (name.to_string(), Some(clean.to_string()));
    }

    if let Some((owner, repo)) = clean.split_once('/') {
        let clean_repo = repo.trim_end_matches(".git");
        if owner == "alya-lang" {
            (clean_repo.to_string(), None)
        } else {
            (
                clean_repo.to_string(),
                Some(format!("https://github.com/{}/{}.git", owner, clean_repo)),
            )
        }
    } else {
        (clean.to_string(), None)
    }
}

pub fn resolve_archive_candidates(
    url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    rev: Option<&str>,
) -> Vec<String> {
    let clean = url.trim_end_matches('/').trim_end_matches(".git");

    // GitHub repository pattern
    if let Some((owner, repo)) = parse_github_owner_repo(clean) {
        let mut candidates = Vec::new();

        if let Some(t) = tag {
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/tags/{}.tar.gz",
                owner, repo, t
            ));
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/tags/{}.zip",
                owner, repo, t
            ));
            let alt = if !t.starts_with('v') && !t.starts_with('V') {
                Some(format!("v{}", t))
            } else {
                t.strip_prefix('v')
                    .or_else(|| t.strip_prefix('V'))
                    .map(|s| s.to_string())
            };
            if let Some(alt_tag) = alt {
                candidates.push(format!(
                    "https://github.com/{}/{}/archive/refs/tags/{}.tar.gz",
                    owner, repo, alt_tag
                ));
                candidates.push(format!(
                    "https://github.com/{}/{}/archive/refs/tags/{}.zip",
                    owner, repo, alt_tag
                ));
            }
        } else if let Some(b) = branch {
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/heads/{}.tar.gz",
                owner, repo, b
            ));
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/heads/{}.zip",
                owner, repo, b
            ));
        } else if let Some(r) = rev {
            candidates.push(format!(
                "https://github.com/{}/{}/archive/{}.tar.gz",
                owner, repo, r
            ));
            candidates.push(format!(
                "https://github.com/{}/{}/archive/{}.zip",
                owner, repo, r
            ));
        } else {
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/heads/main.tar.gz",
                owner, repo
            ));
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/heads/master.tar.gz",
                owner, repo
            ));
            candidates.push(format!(
                "https://github.com/{}/{}/archive/refs/heads/main.zip",
                owner, repo
            ));
        }
        return candidates;
    }

    // GitLab repository pattern
    let gl_sub = clean
        .strip_prefix("https://gitlab.com/")
        .or_else(|| clean.strip_prefix("http://gitlab.com/"));

    if let Some(sub) = gl_sub {
        let parts: Vec<&str> = sub.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[0];
            let repo = parts[1];
            let ref_name = tag.or(branch).or(rev).unwrap_or("main");
            return vec![
                format!(
                    "https://gitlab.com/{}/{}/-/archive/{}/{}-{}.tar.gz",
                    owner, repo, ref_name, repo, ref_name
                ),
                format!(
                    "https://gitlab.com/{}/{}/-/archive/{}/{}-{}.zip",
                    owner, repo, ref_name, repo, ref_name
                ),
            ];
        }
    }

    // Direct HTTP archive URL
    if url.ends_with(".tar.gz") || url.ends_with(".zip") || url.ends_with(".tgz") {
        return vec![url.to_string()];
    }

    Vec::new()
}

pub fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory '{}': {}", dst.display(), e))?;
    let entries = fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory '{}': {}", src.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_contents(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}

/// Downloads one file via curl, wget, or PowerShell (Windows-only fallback).
/// Returns `true` only when the download succeeded and the file is non-empty.
fn download_file(url: &str, dest: &Path) -> bool {
    // 1. Download archive using curl, wget, or PowerShell (suppress noise on probe 404s)
    let mut download_ok = Command::new("curl")
        .args(["-sSL", "-f", url, "-o"])
        .arg(dest)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !download_ok {
        download_ok = Command::new("wget")
            .args(["-q", url, "-O"])
            .arg(dest)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    if !download_ok && cfg!(windows) {
        let ps_script = format!(
            "$ProgressPreference = 'SilentlyContinue'; try {{ Invoke-WebRequest -Uri '{}' -OutFile '{}' -ErrorAction Stop }} catch {{ exit 1 }}",
            url,
            dest.display().to_string().replace('\\', "/")
        );
        download_ok = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }

    download_ok && dest.exists() && fs::metadata(dest).map(|m| m.len()).unwrap_or(0) > 0
}

/// Verifies a file against an expected SHA-256 hex digest (case-insensitive).
pub fn verify_file_sha256(file: &Path, expected_hex: &str) -> bool {
    let expected = expected_hex.trim();
    if expected.is_empty() {
        return false;
    }
    match fs::read(file) {
        Ok(bytes) => sha256_hex(&bytes).eq_ignore_ascii_case(expected),
        Err(_) => false,
    }
}

/// Extracts a downloaded archive into `target_dir` using tar (bsdtar on
/// Windows / GNU tar on Unix). GitHub tag tarballs and curated release
/// assets both carry a single top-level directory, hence
/// `--strip-components 1`. Falls back to PowerShell Expand-Archive for zips
/// on Windows when tar fails.
fn extract_downloaded_archive(
    temp_archive: &Path,
    is_zip: bool,
    target_dir: &Path,
    pkg_name: &str,
    pid: u32,
    millis: u128,
) -> bool {
    // 2. Extract archive using tar (bsdtar on Windows / GNU tar on Unix)
    let tar_ok = Command::new("tar")
        .arg("-xf")
        .arg(temp_archive)
        .args(["--strip-components", "1", "-C"])
        .arg(target_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if tar_ok {
        return true;
    }

    // 3. Fallback on Windows with PowerShell Expand-Archive if tar fails
    if cfg!(windows) && is_zip {
        let temp_dir = env::temp_dir();
        let extract_tmp = temp_dir.join(format!("alya_extract_{}_{}_{}", pkg_name, pid, millis));
        let ps_expand = format!(
            "$ProgressPreference = 'SilentlyContinue'; Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            temp_archive.display().to_string().replace('\\', "/"),
            extract_tmp.display().to_string().replace('\\', "/")
        );
        let ps_ok = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_expand])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ps_ok && extract_tmp.exists() {
            if let Ok(entries) = fs::read_dir(&extract_tmp) {
                let mut found_dir = None;
                for e in entries.flatten() {
                    if e.path().is_dir() {
                        found_dir = Some(e.path());
                        break;
                    }
                }
                let source_folder = found_dir.unwrap_or_else(|| extract_tmp.clone());
                let copy_res = copy_dir_contents(&source_folder, target_dir);
                let _ = fs::remove_dir_all(&extract_tmp);
                if copy_res.is_ok() {
                    return true;
                }
            }
        }
        let _ = fs::remove_dir_all(&extract_tmp);
    }

    false
}

/// Installs a pinned GitHub tag from its curated release asset
/// (`alya-pkg.tar.gz` + detached `.sha256`, published by the standard
/// `release.yml` workflow).
///
/// Returns `Ok(true)` when the asset verified and installed, `Ok(false)`
/// when no asset exists for the tag (caller falls through to the source
/// path silently), and `Err` when an asset exists but is unusable
/// (checksum mismatch, extraction failure) so the caller can warn and fall
/// back instead of failing the install.
pub fn try_download_release_asset(
    name: &str,
    url: &str,
    tag: &str,
    target_dir: &Path,
) -> Result<bool, String> {
    let pairs = resolve_release_asset_urls(url, tag);
    if pairs.is_empty() {
        return Ok(false);
    }

    let temp_dir = env::temp_dir();
    let pid = std::process::id();
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_archive = temp_dir.join(format!("alya_rel_{}_{}_{}.tar.gz", name, pid, millis));
    let temp_sha = temp_dir.join(format!("alya_rel_{}_{}_{}.sha256", name, pid, millis));

    // Detached checksum: standard `sha256sum` text format (`<hex>  <file>`);
    // only the leading hex token is significant.
    let read_expected_hex = || {
        fs::read_to_string(&temp_sha)
            .ok()
            .and_then(|s| s.split_whitespace().next().map(|t| t.to_string()))
            .unwrap_or_default()
    };

    let mut saw_asset = false;
    for (tar_url, sha_url) in &pairs {
        if !download_file(tar_url, &temp_archive) {
            let _ = fs::remove_file(&temp_archive);
            continue;
        }
        if !download_file(sha_url, &temp_sha) {
            // Tarball present but unverifiable on this tag variant; another
            // variant may still be complete.
            let _ = fs::remove_file(&temp_archive);
            let _ = fs::remove_file(&temp_sha);
            saw_asset = true;
            continue;
        }
        let expected = read_expected_hex();
        let _ = fs::remove_file(&temp_sha);
        let actual = fs::read(&temp_archive)
            .map(|b| sha256_hex(&b))
            .unwrap_or_default();
        if expected.is_empty() || !actual.eq_ignore_ascii_case(&expected) {
            let _ = fs::remove_file(&temp_archive);
            return Err(format!(
                "checksum mismatch for release asset '{}' (tag '{}')",
                tar_url, tag
            ));
        }
        println!(
            "  Verified release asset checksum for '{}' (tag '{}')",
            name, tag
        );

        if extract_downloaded_archive(&temp_archive, false, target_dir, name, pid, millis) {
            let _ = fs::remove_file(&temp_archive);
            println!("  Installed '{}' from curated release asset", name);
            return Ok(true);
        }
        let _ = fs::remove_file(&temp_archive);
        return Err(format!("failed to extract release asset '{}'", tar_url));
    }

    let _ = fs::remove_file(&temp_archive);
    let _ = fs::remove_file(&temp_sha);
    if saw_asset {
        return Err(format!(
            "release asset for '{}' (tag '{}') is present but unusable",
            name, tag
        ));
    }
    Ok(false)
}

pub fn try_download_and_extract_archive(
    candidate_urls: &[String],
    target_dir: &Path,
    pkg_name: &str,
) -> Result<(), String> {
    fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Failed to create package directory '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    let temp_dir = env::temp_dir();
    let pid = std::process::id();

    for url in candidate_urls {
        let is_zip = url.ends_with(".zip");
        let ext = if is_zip { "zip" } else { "tar.gz" };
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let temp_archive =
            temp_dir.join(format!("alya_pkg_{}_{}_{}.{}", pkg_name, pid, millis, ext));

        if !download_file(url, &temp_archive) {
            let _ = fs::remove_file(&temp_archive);
            continue;
        }

        if extract_downloaded_archive(&temp_archive, is_zip, target_dir, pkg_name, pid, millis) {
            let _ = fs::remove_file(&temp_archive);
            return Ok(());
        }

        let _ = fs::remove_file(&temp_archive);
    }

    Err(format!(
        "Failed to download and extract archive for '{}' from candidate URLs",
        pkg_name
    ))
}

pub fn try_git_clone(
    url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    target_dir: &Path,
) -> Result<(), String> {
    let mut tags_to_try = Vec::new();
    if let Some(t) = tag {
        tags_to_try.push(t.to_string());
        if !t.starts_with('v') && !t.starts_with('V') {
            tags_to_try.push(format!("v{}", t));
        } else if let Some(stripped) = t.strip_prefix('v').or_else(|| t.strip_prefix('V')) {
            tags_to_try.push(stripped.to_string());
        }
    }

    if !tags_to_try.is_empty() {
        let mut last_err = String::new();
        for t in &tags_to_try {
            let mut cmd = Command::new("git");
            cmd.arg("clone")
                .arg("-q")
                .arg("--depth")
                .arg("1")
                .arg("--branch")
                .arg(t)
                .arg(url)
                .arg(target_dir);
            match cmd.output() {
                Ok(out) if out.status.success() => return Ok(()),
                Ok(out) => {
                    let _ = fs::remove_dir_all(target_dir);
                    let err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    last_err = if !err_msg.is_empty() {
                        err_msg
                    } else {
                        format!(
                            "git clone exited with status {}",
                            out.status.code().unwrap_or(-1)
                        )
                    };
                }
                Err(e) => {
                    let _ = fs::remove_dir_all(target_dir);
                    return Err(format!("could not execute 'git' ({})", e));
                }
            }
        }
        return Err(last_err);
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("-q").arg("--depth").arg("1");
    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(target_dir);

    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let _ = fs::remove_dir_all(target_dir);
            let err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if !err_msg.is_empty() {
                err_msg
            } else {
                format!(
                    "git clone exited with status {}",
                    out.status.code().unwrap_or(-1)
                )
            })
        }
        Err(e) => {
            let _ = fs::remove_dir_all(target_dir);
            Err(format!("could not execute 'git' ({})", e))
        }
    }
}

pub fn fetch_git_or_archive_dependency(
    name: &str,
    url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    rev: Option<&str>,
    target_dir: &Path,
) -> Result<(), String> {
    println!("  Resolving dependency '{}' from {}...", name, url);

    // 0. Pinned GitHub tags prefer the curated release asset (`alya-pkg.tar.gz`
    // + detached `.sha256`). Branch/rev pins, floating refs, and non-GitHub
    // hosts keep today's source path untouched. A missing asset falls through
    // silently; a broken one warns and falls back instead of failing.
    if let Some(t) = tag {
        if branch.is_none() && rev.is_none() {
            match try_download_release_asset(name, url, t, target_dir) {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(e) => println!("  Notice: {} — falling back to source.", e),
            }
        }
    }

    // 1. Try Git clone first if git CLI is installed
    let git_err = match try_git_clone(url, tag, branch, target_dir) {
        Ok(()) => {
            if let Some(r) = rev {
                let checkout = Command::new("git")
                    .current_dir(target_dir)
                    .args(["checkout", "-q", r])
                    .output();
                if let Ok(out) = checkout {
                    if !out.status.success() {
                        let _ = Command::new("git")
                            .current_dir(target_dir)
                            .args(["fetch", "-q", "--unshallow"])
                            .output();
                        let _ = Command::new("git")
                            .current_dir(target_dir)
                            .args(["checkout", "-q", r])
                            .output();
                    }
                }
            }

            // Record commit SHA before deleting .git so local package manager can track branch updates accurately
            let rev_sha = Command::new("git")
                .current_dir(target_dir)
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        if !s.is_empty() {
                            Some(s)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
            if let Some(sha) = rev_sha {
                let _ = fs::write(target_dir.join(".alya-rev"), sha);
            }

            // Strip .git directory so cached packages remain clean and lightweight
            let git_dir = target_dir.join(".git");
            if git_dir.exists() {
                let _ = fs::remove_dir_all(&git_dir);
            }
            return Ok(());
        }
        Err(e) => e,
    };

    // 2. If git is missing or failed, fall back to automatic HTTP archive download
    let candidate_urls = resolve_archive_candidates(url, tag, branch, rev);
    if candidate_urls.is_empty() {
        return Err(format!(
            "Git clone failed ({}) and no HTTP archive candidate URL could be resolved for '{}'",
            git_err, url
        ));
    }

    println!("  Notice: Git unavailable ({}).", git_err);
    println!(
        "  Attempting automatic HTTP archive download fallback for '{}'...",
        name
    );

    match try_download_and_extract_archive(&candidate_urls, target_dir, name) {
        Ok(()) => {
            if let Some(r) = rev {
                let _ = fs::write(target_dir.join(".alya-rev"), r);
            } else if let Some(b) = branch {
                if let Some(sha) = query_remote_branch_head(url, b) {
                    let _ = fs::write(target_dir.join(".alya-rev"), sha);
                }
            }
            println!(
                "  ✓ Successfully downloaded and unpacked '{}' archive without Git",
                name
            );
            Ok(())
        }
        Err(fallback_err) => {
            let _ = fs::remove_dir_all(target_dir);
            Err(format!(
                "Failed to fetch package '{}' from {}:\n  Git error: {}\n  Archive download error: {}",
                name, url, git_err, fallback_err
            ))
        }
    }
}

pub fn copy_dir_all(src: &Path, dst: &Path, skip_git: bool) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory '{}': {}", dst.display(), e))?;
    }
    let entries = fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory '{}': {}", src.display(), e))?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        if skip_git && file_name == ".git" {
            continue;
        }
        let target_path = dst.join(&file_name);
        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &target_path, skip_git)?;
        } else if entry_path.is_file() {
            fs::copy(&entry_path, &target_path).map_err(|e| {
                format!(
                    "Failed to copy file '{}' to '{}': {}",
                    entry_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

pub fn parse_semver(v: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let clean = v.trim().trim_start_matches(['v', 'V']);
    let (num_part, pre_part) = match clean.split_once('-') {
        Some((n, p)) => (n, Some(p.to_string())),
        None => (clean, None),
    };
    let parts: Vec<&str> = num_part.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let major = parts[0].parse::<u64>().ok()?;
    let minor = if parts.len() > 1 {
        parts[1].parse::<u64>().ok()?
    } else {
        0
    };
    let patch = if parts.len() > 2 {
        parts[2].parse::<u64>().ok()?
    } else {
        0
    };
    Some((major, minor, patch, pre_part))
}

pub fn compare_semver(v1: &str, v2: &str) -> std::cmp::Ordering {
    match (parse_semver(v1), parse_semver(v2)) {
        (Some((maj1, min1, pat1, pre1)), Some((maj2, min2, pat2, pre2))) => maj1
            .cmp(&maj2)
            .then(min1.cmp(&min2))
            .then(pat1.cmp(&pat2))
            .then_with(|| match (pre1, pre2) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(p1), Some(p2)) => p1.cmp(&p2),
            }),
        _ => v1.cmp(v2),
    }
}

pub fn query_remote_tags(url: &str) -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "-q", url])
        .output();
    let mut tags = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(ref_part) = line.split_whitespace().nth(1) {
                    if let Some(tag) = ref_part.strip_prefix("refs/tags/") {
                        if !tag.ends_with("^{}") {
                            tags.push(tag.to_string());
                        }
                    }
                }
            }
        }
    }
    tags
}

/// Resolves a tag name to its commit SHA via `git ls-remote`.
///
/// Prefers the `^{}` dereferenced commit for annotated tags so the result
/// is always the commit a checkout would land on. Returns `None` offline
/// or when the tag does not exist; callers must fall back gracefully.
pub fn query_tag_rev(url: &str, tag: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "-q", url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let want_peeled = format!("refs/tags/{tag}^{{}}");
    let want_plain = format!("refs/tags/{tag}");
    let mut plain_sha = None;
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let sha = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        match parts.next() {
            Some(r) if r == want_peeled => return Some(sha.to_string()),
            Some(r) if r == want_plain => plain_sha = Some(sha.to_string()),
            _ => {}
        }
    }
    plain_sha
}

pub fn query_remote_branch_head(url: &str, branch: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", "-q", url, branch])
        .output();
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(sha) = line.split_whitespace().next() {
                    return Some(sha.to_string());
                }
            }
        }
    }
    None
}

pub fn find_latest_semver_tag<'a>(tags: &'a [String]) -> Option<&'a str> {
    let mut semver_tags: Vec<&'a str> = tags
        .iter()
        .map(|s| s.as_str())
        .filter(|t| parse_semver(t).is_some())
        .collect();
    semver_tags.sort_by(|a, b| compare_semver(a, b));
    semver_tags.last().copied()
}

pub fn semver_major(v: &str) -> Option<u64> {
    let clean = v.trim().trim_start_matches(['^', '~', '=', 'v', 'V', '>']);
    parse_semver(clean).map(|(maj, _, _, _)| maj)
}

pub fn is_semver_compatible(v1: &str, v2: &str) -> bool {
    let c1 = v1.trim().trim_start_matches(['^', '~', '=', 'v', 'V', '>']);
    let c2 = v2.trim().trim_start_matches(['^', '~', '=', 'v', 'V', '>']);
    match (parse_semver(c1), parse_semver(c2)) {
        (Some((maj1, min1, _, _)), Some((maj2, min2, _, _))) => {
            if maj1 != maj2 {
                return false;
            }
            if maj1 == 0 {
                min1 == min2
            } else {
                true
            }
        }
        _ => c1 == c2,
    }
}

pub fn coalesce_semver_versions<'a>(v1: &'a str, v2: &'a str) -> Result<&'a str, String> {
    if !is_semver_compatible(v1, v2) {
        return Err(format!(
            "Incompatible SemVer versions '{}' and '{}' cannot be coalesced",
            v1, v2
        ));
    }
    let c1 = v1.trim().trim_start_matches(['^', '~', '=', 'v', 'V', '>']);
    let c2 = v2.trim().trim_start_matches(['^', '~', '=', 'v', 'V', '>']);
    if compare_semver(c1, c2).is_ge() {
        Ok(v1)
    } else {
        Ok(v2)
    }
}
