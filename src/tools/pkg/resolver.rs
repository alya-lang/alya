use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

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
    let gh_sub = clean
        .strip_prefix("https://github.com/")
        .or_else(|| clean.strip_prefix("http://github.com/"))
        .or_else(|| clean.strip_prefix("git@github.com:"));

    if let Some(sub) = gh_sub {
        let parts: Vec<&str> = sub.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[0];
            let repo = parts[1];
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

        // 1. Download archive using curl, wget, or PowerShell (suppress noise on probe 404s)
        let mut download_ok = Command::new("curl")
            .args(["-sSL", "-f", url, "-o"])
            .arg(&temp_archive)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !download_ok {
            download_ok = Command::new("wget")
                .args(["-q", url, "-O"])
                .arg(&temp_archive)
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
                temp_archive.display().to_string().replace('\\', "/")
            );
            download_ok = Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }

        if !download_ok
            || !temp_archive.exists()
            || fs::metadata(&temp_archive).map(|m| m.len()).unwrap_or(0) == 0
        {
            let _ = fs::remove_file(&temp_archive);
            continue;
        }

        // 2. Extract archive using tar (bsdtar on Windows / GNU tar on Unix)
        let tar_ok = Command::new("tar")
            .arg("-xf")
            .arg(&temp_archive)
            .args(["--strip-components", "1", "-C"])
            .arg(target_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if tar_ok {
            let _ = fs::remove_file(&temp_archive);
            return Ok(());
        }

        // 3. Fallback on Windows with PowerShell Expand-Archive if tar fails
        if cfg!(windows) && is_zip {
            let extract_tmp =
                temp_dir.join(format!("alya_extract_{}_{}_{}", pkg_name, pid, millis));
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
                    let _ = fs::remove_file(&temp_archive);
                    if copy_res.is_ok() {
                        return Ok(());
                    }
                }
            }
            let _ = fs::remove_dir_all(&extract_tmp);
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

pub fn update_git_dependency(tag: Option<&str>, branch: Option<&str>, target_dir: &Path) {
    if !target_dir.join(".git").exists() {
        return;
    }
    let _ = Command::new("git")
        .current_dir(target_dir)
        .args(["fetch", "-q", "--depth", "1"])
        .output();
    if let Some(t) = tag {
        let _ = Command::new("git")
            .current_dir(target_dir)
            .args(["checkout", "-q", t])
            .output();
    } else if let Some(b) = branch {
        let _ = Command::new("git")
            .current_dir(target_dir)
            .args(["checkout", "-q", b])
            .output();
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

    // 1. Try Git clone first if git CLI is installed
    let git_err = match try_git_clone(url, tag, branch, target_dir) {
        Ok(()) => {
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
