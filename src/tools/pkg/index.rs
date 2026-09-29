//! Static package index (registry) client.
//!
//! Index layout (static files, no server):
//! ```text
//! <base>/packages/<name>.json
//! ```
//! ```json
//! {
//!   "name": "http",
//!   "repository": "https://github.com/alya-lang/http",
//!   "versions": [
//!     {
//!       "version": "0.2.0",
//!       "tag": "v0.2.0",
//!       "checksum": "sha256:<hex of alya-pkg.tar.gz>",
//!       "requires_alya": "0.0.19",
//!       "tarball": "https://... (optional override)",
//!       "yanked": false
//!     }
//!   ]
//! }
//! ```
//! The base URL comes from `ALYA_REGISTRY_INDEX` (http/https/file) and
//! defaults to `https://raw.githubusercontent.com/alya-lang/index/main`.
//! Until that repository exists every lookup degrades to `None` and the
//! installer silently keeps its git-based flow — shipping this client
//! changes no behavior by itself.
//!
//! Trust model: the index checksum is the trust root for tarballs fetched
//! via explicit `tarball` URLs. For derived GitHub release-asset URLs the
//! existing detached `.sha256` verification still applies; an index
//! checksum, when present, is additionally enforced.
//!
//! Requirement syntax (`version_satisfies_req`): bare/`=`/`v`-prefixed
//! exact, `^` compatible, `~` minor-pinned, `*` any. Anything else is a
//! hard error (fail loudly, never guess).

use std::fs;
use std::path::Path;
use std::process::Command;

pub const DEFAULT_INDEX_BASE: &str = "https://raw.githubusercontent.com/alya-lang/index/main";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexVersion {
    pub version: String,
    pub tag: String,
    pub checksum: Option<String>,
    pub requires_alya: Option<String>,
    pub tarball: Option<String>,
    pub yanked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexPackage {
    pub name: String,
    pub repository: Option<String>,
    pub versions: Vec<IndexVersion>,
}

pub fn index_base_url() -> String {
    if let Ok(raw) = std::env::var("ALYA_REGISTRY_INDEX") {
        let trimmed = raw.trim().trim_end_matches('/').to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    DEFAULT_INDEX_BASE.to_string()
}

fn index_file_url(name: &str) -> String {
    format!("{}/packages/{}.json", index_base_url(), name)
}

/// Fetches URL text: `file://` reads directly, http(s) tries
/// curl → wget → PowerShell with short timeouts. Silent `None` on any
/// failure (the caller falls back to git-based resolution).
pub fn fetch_url_text(url: &str) -> Option<String> {
    if let Some(path) = url.strip_prefix("file://") {
        return fs::read_to_string(path)
            .ok()
            .filter(|s| !s.trim().is_empty());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    // 1. curl
    if let Ok(output) = Command::new("curl")
        .args(["-sSL", "--max-time", "15", url])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }
    // 2. wget
    if let Ok(output) = Command::new("wget")
        .args(["-qO-", "--timeout=15", url])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }
    // 3. PowerShell (Windows hosts without curl/wget)
    if cfg!(target_os = "windows") {
        let script = format!(
            "$ProgressPreference = 'SilentlyContinue'; (Invoke-WebRequest -Uri '{}' -UseBasicParsing -TimeoutSec 15).Content",
            url.replace('\'', "''")
        );
        if let Ok(output) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
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

fn get_str(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key)?
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parses one `packages/<name>.json` document. Strict on shape (unknown
/// versions entries are skipped, malformed documents rejected) so a
/// corrupt index fails loudly instead of resolving garbage.
pub fn parse_index_package(text: &str) -> Option<IndexPackage> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let name = get_str(&v, "name")?;
    let repository = get_str(&v, "repository");
    let mut versions = Vec::new();
    for entry in v.get("versions")?.as_array()? {
        let version = get_str(entry, "version")?;
        let tag = get_str(entry, "tag").unwrap_or_else(|| format!("v{}", version));
        let checksum = get_str(entry, "checksum").and_then(|c| {
            let hex = c
                .strip_prefix("sha256:")
                .map(|s| s.to_string())
                .unwrap_or(c);
            if hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
                Some(format!("sha256:{}", hex.to_lowercase()))
            } else {
                None
            }
        });
        versions.push(IndexVersion {
            version,
            tag,
            checksum,
            requires_alya: get_str(entry, "requires_alya"),
            tarball: get_str(entry, "tarball"),
            yanked: entry
                .get("yanked")
                .and_then(|y| y.as_bool())
                .unwrap_or(false),
        });
    }
    if versions.is_empty() {
        return None;
    }
    Some(IndexPackage {
        name,
        repository,
        versions,
    })
}

pub fn fetch_package_index(name: &str) -> Option<IndexPackage> {
    fetch_url_text(&index_file_url(name))
        .and_then(|text| parse_index_package(&text))
        .filter(|pkg| pkg.name == name)
}

/// Splits `major.minor.patch[-pre]` (leading `v` tolerated).
fn parse_req_version(s: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let clean = s.trim().trim_start_matches(['v', 'V']);
    let (core, pre) = match clean.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (clean, None),
    };
    let mut parts = core.split('.');
    let maj: u64 = parts.next()?.trim().parse().ok()?;
    let min: u64 = parts.next().unwrap_or("0").trim().parse().ok()?;
    let pat: u64 = parts.next().unwrap_or("0").trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((maj, min, pat, pre))
}

/// Whether concrete `version` satisfies requirement `req`.
/// Supported: `*`, bare/`=`/`v`-exact, `^` compatible (same major, or same
/// minor when major is 0), `~` minor-pinned. Pre-releases only satisfy an
/// identical pre-release requirement. Anything else errors.
pub fn version_satisfies_req(req: &str, version: &str) -> Result<bool, String> {
    let req = req.trim();
    if req == "*" || req.is_empty() {
        return Ok(true);
    }
    let (op, core) = if let Some(rest) = req.strip_prefix('^') {
        ("^", rest)
    } else if let Some(rest) = req.strip_prefix('~') {
        ("~", rest)
    } else if let Some(rest) = req.strip_prefix('=') {
        ("=", rest)
    } else {
        ("=", req)
    };
    let (rmaj, rmin, rpat, rpre) = parse_req_version(core).ok_or_else(|| {
        format!(
            "Invalid version requirement '{}': expected '*', '1.2.3', '^1.2.3' or '~1.2.3'",
            req
        )
    })?;
    let (vmaj, vmin, vpat, vpre) = parse_req_version(version)
        .ok_or_else(|| format!("Invalid version '{}' for requirement '{}'", version, req))?;
    if rpre.is_some() || vpre.is_some() {
        return Ok(rmaj == vmaj && rmin == vmin && rpat == vpat && rpre == vpre);
    }
    let ok = match op {
        "=" => vmaj == rmaj && vmin == rmin && vpat == rpat,
        "^" => {
            if rmaj > 0 {
                vmaj == rmaj
            } else if rmin > 0 {
                vmaj == 0 && vmin == rmin
            } else {
                vmaj == 0 && vmin == 0 && vpat == rpat
            }
        }
        "~" => vmaj == rmaj && vmin == rmin,
        _ => false,
    };
    Ok(ok)
}

/// Highest non-yanked version satisfying `req`, or `None` when the index
/// lists no match (caller falls back to git-based resolution).
pub fn select_index_version<'a>(
    pkg: &'a IndexPackage,
    req: &str,
) -> Result<Option<&'a IndexVersion>, String> {
    let mut best: Option<&'a IndexVersion> = None;
    for v in &pkg.versions {
        if v.yanked {
            continue;
        }
        if !version_satisfies_req(req, &v.version)? {
            continue;
        }
        let better = match &best {
            None => true,
            Some(b) => {
                super::resolver::compare_semver(&v.version, &b.version)
                    == std::cmp::Ordering::Greater
            }
        };
        if better {
            best = Some(v);
        }
    }
    Ok(best)
}

/// Tarball URL for an index version: explicit override, else the GitHub
/// release-asset convention derived from the package repository + tag.
pub fn index_tarball_url(pkg: &IndexPackage, version: &IndexVersion) -> Option<String> {
    if let Some(url) = version.tarball.as_deref() {
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    let repo = pkg.repository.as_deref()?;
    let urls = super::resolver::resolve_release_asset_urls(repo, &version.tag);
    urls.into_iter().next().map(|(tar, _)| tar)
}

/// An index resolution: concrete tag plus optional explicit tarball with
/// its inline checksum (both present or neither).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexPick {
    pub tag: String,
    pub version: String,
    pub tarball: Option<String>,
    pub checksum_hex: Option<String>,
}

/// Resolves requirement `req` against the index: highest non-yanked
/// satisfying version, or `None` when the index is silent (caller falls
/// back to git-based resolution). Malformed requirements are hard errors.
pub fn select_version(name: &str, req: &str) -> Result<Option<IndexPick>, String> {
    let Some(pkg) = fetch_package_index(name) else {
        return Ok(None);
    };
    let Some(selected) = select_index_version(&pkg, req)? else {
        return Ok(None);
    };
    let explicit = match (&selected.tarball, &selected.checksum) {
        (Some(tar), Some(sum)) if !tar.is_empty() => {
            let hex = sum
                .strip_prefix("sha256:")
                .map(|s| s.to_string())
                .unwrap_or_else(|| sum.clone());
            Some((tar.clone(), hex))
        }
        _ => None,
    };
    Ok(Some(IndexPick {
        tag: selected.tag.clone(),
        version: selected.version.clone(),
        tarball: explicit.clone().map(|(t, _)| t),
        checksum_hex: explicit.map(|(_, h)| h),
    }))
}

/// Installs an explicit-tarball index entry with strict inline-checksum
/// verification (the index is the trust root here).
/// Returns `Ok(true)` on success, `Ok(false)` when the tarball is
/// unreachable (caller falls back to git), `Err` on checksum mismatch
/// or extraction failure (fail safe, never install garbage).
pub(crate) fn install_index_tarball(
    name: &str,
    tar_url: &str,
    expected_hex: &str,
    tag: &str,
    target_dir: &Path,
) -> Result<bool, String> {
    use super::hash::sha256_hex;
    use super::resolver::{download_file, extract_downloaded_archive};
    use std::path::PathBuf;

    std::fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Failed to create package directory '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_archive: PathBuf =
        temp_dir.join(format!("alya_idx_{}_{}_{}.tar.gz", name, pid, millis));

    if !download_file(tar_url, &temp_archive) {
        let _ = std::fs::remove_file(&temp_archive);
        return Ok(false);
    }
    let actual = std::fs::read(&temp_archive).map(|b| sha256_hex(&b));
    let matches = actual
        .as_deref()
        .map(|a| a.eq_ignore_ascii_case(expected_hex))
        .unwrap_or(false);
    if !matches {
        let _ = std::fs::remove_file(&temp_archive);
        return Err(format!(
            "checksum mismatch for index tarball '{}' (tag '{}')",
            tar_url, tag
        ));
    }
    println!("  Verified index checksum for '{}' (tag '{}')", name, tag);
    if extract_downloaded_archive(&temp_archive, false, target_dir, name, pid, millis) {
        let _ = std::fs::remove_file(&temp_archive);
        let _ = std::fs::write(
            target_dir.join(".alya-asset"),
            format!("{} {}", tar_url, expected_hex),
        );
        println!("  Installed '{}' from index tarball", name);
        return Ok(true);
    }
    let _ = std::fs::remove_file(&temp_archive);
    Err(format!("failed to extract index tarball '{}'", tar_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "name": "http",
        "repository": "https://github.com/alya-lang/http",
        "versions": [
            {"version": "0.1.0", "tag": "v0.1.0", "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000"},
            {"version": "0.2.0", "tag": "v0.2.0", "checksum": "0000000000000000000000000000000000000000000000000000000000000001", "requires_alya": "0.0.19"},
            {"version": "0.3.0", "tag": "v0.3.0", "yanked": true}
        ]
    }"#;

    #[test]
    fn parses_index_document() {
        let pkg = parse_index_package(SAMPLE).expect("parse");
        assert_eq!(pkg.name, "http");
        assert_eq!(pkg.versions.len(), 3);
        assert_eq!(pkg.versions[1].tag, "v0.2.0");
        assert_eq!(
            pkg.versions[1].checksum.as_deref(),
            Some("sha256:0000000000000000000000000000000000000000000000000000000000000001")
        );
        assert!(pkg.versions[2].yanked);
        // Malformed documents are rejected, not half-parsed.
        assert!(parse_index_package("{}").is_none());
        assert!(parse_index_package("{\"name\":\"x\"}").is_none());
        assert!(parse_index_package("not json").is_none());
    }

    #[test]
    fn requirement_matching() {
        assert_eq!(version_satisfies_req("*", "1.2.3"), Ok(true));
        assert_eq!(version_satisfies_req("1.2.3", "1.2.3"), Ok(true));
        assert_eq!(version_satisfies_req("=1.2.3", "1.2.4"), Ok(false));
        assert_eq!(version_satisfies_req("v1.2.3", "1.2.3"), Ok(true));
        assert_eq!(version_satisfies_req("^1.2.0", "1.9.0"), Ok(true));
        assert_eq!(version_satisfies_req("^1.2.0", "2.0.0"), Ok(false));
        assert_eq!(version_satisfies_req("^0.2.0", "0.2.9"), Ok(true));
        assert_eq!(version_satisfies_req("^0.2.0", "0.3.0"), Ok(false));
        assert_eq!(version_satisfies_req("^0.0.1", "0.0.2"), Ok(false));
        assert_eq!(version_satisfies_req("~1.2.0", "1.2.9"), Ok(true));
        assert_eq!(version_satisfies_req("~1.2.0", "1.3.0"), Ok(false));
        assert_eq!(
            version_satisfies_req("1.0.0-alpha", "1.0.0-alpha"),
            Ok(true)
        );
        assert_eq!(version_satisfies_req("1.0.0", "1.0.0-alpha"), Ok(false));
        assert!(version_satisfies_req(">=1.0", "1.0.0").is_err());
        assert!(version_satisfies_req("bogus", "1.0.0").is_err());
    }

    #[test]
    fn selection_prefers_max_stable() {
        let pkg = parse_index_package(SAMPLE).expect("parse");
        // Yanked 0.3.0 is skipped even though it is the max.
        assert_eq!(
            select_index_version(&pkg, "*")
                .expect("select")
                .map(|v| v.version.as_str()),
            Some("0.2.0")
        );
        assert_eq!(
            select_index_version(&pkg, "^0.1.0")
                .expect("select")
                .map(|v| v.version.as_str()),
            Some("0.1.0")
        );
        assert!(select_index_version(&pkg, "^2.0.0")
            .expect("select")
            .is_none());
    }

    #[test]
    fn tarball_url_derivation() {
        let pkg = parse_index_package(SAMPLE).expect("parse");
        let v = &pkg.versions[1];
        let url = index_tarball_url(&pkg, v).expect("url");
        assert!(url.contains("alya-lang/http"));
        assert!(url.contains("v0.2.0"));
        assert!(url.ends_with("alya-pkg.tar.gz"));
    }
}
