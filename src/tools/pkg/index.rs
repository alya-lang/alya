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
//!       "requires_alya": "0.0.20",
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

/// Index layouts. `flat-v1` is `packages/<name>.json`; `sharded-v2`
/// is `packages/<aa>/<name>.json` (`aa` = first two lowercase chars,
/// `1/` and `2/` for one- and two-letter names, mirroring cargo's
/// scheme). Unknown layouts are rejected (fail safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexLayout {
    Flat,
    Sharded,
}

pub fn parse_index_layout(text: &str) -> Option<IndexLayout> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    match v.get("layout")?.as_str()? {
        "flat-v1" => Some(IndexLayout::Flat),
        "sharded-v2" => Some(IndexLayout::Sharded),
        _ => None,
    }
}

/// Shard directory for a package name under `sharded-v2`.
pub fn shard_dir(name: &str) -> String {
    let lower: String = name.chars().flat_map(|c| c.to_lowercase()).collect();
    let chars: Vec<char> = lower.chars().collect();
    match chars.len() {
        0 => "_".to_string(),
        1 => "1".to_string(),
        2 => "2".to_string(),
        _ => chars[..2].iter().collect(),
    }
}

pub fn package_index_path(name: &str) -> Option<String> {
    match index_layout() {
        Some(IndexLayout::Sharded) => Some(format!("packages/{}/{}.json", shard_dir(name), name)),
        // Unknown/missing root: assume flat-v1 (backward compatible with
        // indexes predating the root document).
        _ => Some(format!("packages/{}.json", name)),
    }
}

fn index_root_url() -> String {
    format!("{}/index.json", index_base_url())
}

/// Reads the root layout document (cached like package docs). `None`
/// means flat-v1 by default — never an error by itself.
pub fn index_layout() -> Option<IndexLayout> {
    let url = index_root_url();
    if url.starts_with("file://") {
        let text = fetch_url_text(&url)?;
        // A file:// root that exists but misdeclares is a real error
        // signal only if unparseable as JSON at all; unknown layouts
        // fall back to flat (forward compatible reads).
        return Some(parse_index_layout(&text).unwrap_or(IndexLayout::Flat));
    }
    let name = "index-root";
    if let Some(cached) = read_cached_index_raw(name, false) {
        if let Some(layout) = parse_index_layout(&cached) {
            return Some(layout);
        }
    }
    match fetch_url_text(&url) {
        Some(text) => match parse_index_layout(&text) {
            Some(layout) => {
                store_cached_index(name, &text);
                Some(layout)
            }
            None => read_cached_raw_fallback(name),
        },
        None => read_cached_raw_fallback(name),
    }
}

fn read_cached_raw_fallback(name: &str) -> Option<IndexLayout> {
    read_cached_index_raw(name, true).and_then(|text| parse_index_layout(&text))
}

/// Raw cached text: fresh-only by default, any age when stale-allowed.
fn read_cached_index_raw(name: &str, allow_stale: bool) -> Option<String> {
    let (json_path, meta_path) = index_cache_paths(name)?;
    let raw = std::fs::read_to_string(&json_path).ok()?;
    if !allow_stale {
        let meta_raw = std::fs::read_to_string(&meta_path).ok()?;
        let meta: serde_json::Value = serde_json::from_str(&meta_raw).ok()?;
        let fetched_at = meta.get("fetched_at")?.as_u64()?;
        if now_unix_secs().saturating_sub(fetched_at) > cache_ttl_secs() {
            return None;
        }
    }
    Some(raw)
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
    let path = package_index_path(name)?;
    let url = format!("{}/{}", index_base_url(), path);
    // Local files are the source of truth: always read fresh, never cache.
    if url.starts_with("file://") {
        return fetch_url_text(&url)
            .and_then(|text| parse_index_package(&text))
            .filter(|pkg| pkg.name == name);
    }
    // Fresh-enough cache first (no network).
    if let Some(pkg) = read_cached_index(name, false) {
        return Some(pkg);
    }
    // Network, then stale fallback (any age beats failing the install).
    // Only validated documents touch the cache: error pages must never
    // poison it (a poisoned cache would turn one outage into a sticky one).
    match fetch_url_text(&url) {
        Some(text) => match parse_index_package(&text).filter(|pkg| pkg.name == name) {
            Some(pkg) => {
                store_cached_index(name, &text);
                Some(pkg)
            }
            None => read_cached_index(name, true),
        },
        None => read_cached_index(name, true),
    }
}

/// Cache TTL in seconds (`ALYA_REGISTRY_TTL`, default one hour).
/// Local files bypass the cache entirely.
fn cache_ttl_secs() -> u64 {
    std::env::var("ALYA_REGISTRY_TTL")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(3600)
}

fn cache_safe_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn index_cache_paths(name: &str) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let dir = super::cache::get_global_cache_dir()?.join("index");
    let safe = cache_safe_name(name);
    Some((
        dir.join(format!("{}.json", safe)),
        dir.join(format!("{}.meta.json", safe)),
    ))
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Reads the cached document: fresh-only by default, any age when
/// `allow_stale` (offline fallback).
fn read_cached_index(name: &str, allow_stale: bool) -> Option<IndexPackage> {
    let raw = read_cached_index_raw(name, allow_stale)?;
    parse_index_package(&raw).filter(|pkg| pkg.name == name)
}

fn store_cached_index(name: &str, text: &str) {
    let Some((json_path, meta_path)) = index_cache_paths(name) else {
        return;
    };
    if let Some(parent) = json_path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if std::fs::write(&json_path, text).is_err() {
        return;
    }
    let meta = serde_json::json!({"v": 1, "fetched_at": now_unix_secs()});
    let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
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
/// minor when major is 0), `~` minor-pinned, and comma-separated
/// AND-combinations with `>`, `>=`, `<`, `<=` (e.g. `">=1.2.0, <2.0.0"`).
/// Pre-releases only satisfy an identical pre-release requirement.
/// Anything else errors.
pub fn version_satisfies_req(req: &str, version: &str) -> Result<bool, String> {
    let req = req.trim();
    if req == "*" || req.is_empty() {
        return Ok(true);
    }
    let (vmaj, vmin, vpat, vpre) = parse_req_version(version)
        .ok_or_else(|| format!("Invalid version '{}' for requirement '{}'", version, req))?;
    for part in req.split(',') {
        if !satisfies_comparator(part.trim(), vmaj, vmin, vpat, vpre.as_deref())? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn satisfies_comparator(
    part: &str,
    vmaj: u64,
    vmin: u64,
    vpat: u64,
    vpre: Option<&str>,
) -> Result<bool, String> {
    let invalid = || {
        format!(
            "Invalid version requirement '{}': expected '*', '1.2.3', '^1.2.3', '~1.2.3' or '>=1.0.0, <2.0.0'",
            part
        )
    };
    let (op, core) = if let Some(rest) = part.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = part.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = part.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = part.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = part.strip_prefix('^') {
        ("^", rest)
    } else if let Some(rest) = part.strip_prefix('~') {
        ("~", rest)
    } else if let Some(rest) = part.strip_prefix('=') {
        ("=", rest)
    } else {
        ("=", part)
    };
    // `=v1.2.3` and `==1.2.3` spellings.
    let core = core.strip_prefix('=').unwrap_or(core);
    let (rmaj, rmin, rpat, rpre) = parse_req_version(core).ok_or_else(invalid)?;
    if rpre.is_some() || vpre.is_some() {
        return Ok(rmaj == vmaj && rmin == vmin && rpat == vpat && rpre.as_deref() == vpre);
    }
    let cmp = (vmaj, vmin, vpat).cmp(&(rmaj, rmin, rpat));
    let ok = match op {
        "=" => cmp == std::cmp::Ordering::Equal,
        ">" => cmp == std::cmp::Ordering::Greater,
        ">=" => cmp != std::cmp::Ordering::Less,
        "<" => cmp == std::cmp::Ordering::Less,
        "<=" => cmp != std::cmp::Ordering::Greater,
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
    select_index_version_inner(pkg, req, true)
}

/// Highest version satisfying `req`, yanked or not. Used to notice when
/// the yank filter changed the outcome so callers can warn instead of
/// silently switching versions.
pub fn select_index_version_including_yanked<'a>(
    pkg: &'a IndexPackage,
    req: &str,
) -> Result<Option<&'a IndexVersion>, String> {
    select_index_version_inner(pkg, req, false)
}

fn select_index_version_inner<'a>(
    pkg: &'a IndexPackage,
    req: &str,
    skip_yanked: bool,
) -> Result<Option<&'a IndexVersion>, String> {
    let mut best: Option<&'a IndexVersion> = None;
    for v in &pkg.versions {
        if skip_yanked && v.yanked {
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

/// Highest non-yanked version in the index, if any. Used by `update` to
/// filter yanked tags out of the LATEST computation.
pub fn select_latest_stable(pkg: &IndexPackage) -> Option<&IndexVersion> {
    let mut best: Option<&IndexVersion> = None;
    for v in &pkg.versions {
        if v.yanked {
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
    best
}

/// Exact version lookup regardless of yank state (`v` prefix optional on
/// either side). Used to detect yanked pins/locks so callers can warn.
pub fn lookup_index_version<'a>(pkg: &'a IndexPackage, version: &str) -> Option<&'a IndexVersion> {
    let want = version.trim().trim_start_matches(['v', 'V']);
    pkg.versions
        .iter()
        .find(|v| v.version.trim().trim_start_matches(['v', 'V']) == want)
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
            {"version": "0.2.0", "tag": "v0.2.0", "checksum": "0000000000000000000000000000000000000000000000000000000000000001", "requires_alya": "0.0.20"},
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
        assert_eq!(version_satisfies_req(">=1.2.0, <2.0.0", "1.9.9"), Ok(true));
        assert_eq!(version_satisfies_req(">=1.2.0, <2.0.0", "2.0.0"), Ok(false));
        assert_eq!(version_satisfies_req(">=1.2.0, <2.0.0", "1.1.9"), Ok(false));
        assert_eq!(version_satisfies_req(">1.0.0", "1.0.1"), Ok(true));
        assert_eq!(version_satisfies_req("<=2.0.0", "2.0.0"), Ok(true));
        assert_eq!(version_satisfies_req("==1.2.3", "1.2.3"), Ok(true));
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
    fn latest_stable_and_exact_lookup() {
        let pkg = parse_index_package(SAMPLE).expect("parse");
        // Yanked 0.3.0 never surfaces as latest.
        assert_eq!(
            select_latest_stable(&pkg).map(|v| v.version.as_str()),
            Some("0.2.0")
        );
        // Exact lookup finds entries regardless of yank state, `v` optional.
        assert_eq!(
            lookup_index_version(&pkg, "0.3.0").map(|v| v.yanked),
            Some(true)
        );
        assert_eq!(
            lookup_index_version(&pkg, "v0.1.0").map(|v| v.tag.as_str()),
            Some("v0.1.0")
        );
        assert!(lookup_index_version(&pkg, "9.9.9").is_none());
        // Including-yanked selection sees the yanked max (for warnings).
        assert_eq!(
            select_index_version_including_yanked(&pkg, "*")
                .expect("select")
                .map(|v| v.version.as_str()),
            Some("0.3.0")
        );
        assert_eq!(
            select_index_version_including_yanked(&pkg, "^0.1.0")
                .expect("select")
                .map(|v| v.version.as_str()),
            Some("0.1.0")
        );
    }

    #[test]
    fn sharded_layout_resolution_offline() {
        // ALYA_REGISTRY_INDEX is process-global: hold the registry lock
        // across the whole set/fetch/restore window (parallel test
        // threads otherwise observe `file://` mid-flight).
        let _env_guard = crate::tools::pkg::lock_registry_env();
        let base = std::env::temp_dir().join(format!("alya_test_idxshard_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let pkgs = base.join("packages").join("sh");
        std::fs::create_dir_all(&pkgs).unwrap();
        std::fs::write(base.join("index.json"), "{\"layout\": \"sharded-v2\"}\n").unwrap();
        std::fs::write(
            pkgs.join("shardpkg.json"),
            "{\"name\": \"shardpkg\", \"versions\": [{\"version\": \"2.0.0\", \"tag\": \"v2.0.0\"}]}",
        )
        .unwrap();
        assert_eq!(shard_dir("shardpkg"), "sh");
        assert_eq!(shard_dir("a"), "1");
        assert_eq!(shard_dir("ab"), "2");
        assert_eq!(
            parse_index_layout("{\"layout\": \"sharded-v2\"}"),
            Some(IndexLayout::Sharded)
        );
        assert_eq!(
            parse_index_layout("{\"layout\": \"flat-v1\"}"),
            Some(IndexLayout::Flat)
        );
        assert_eq!(parse_index_layout("{\"layout\": \"x\"}"), None);

        let prev = std::env::var("ALYA_REGISTRY_INDEX").ok();
        std::env::set_var(
            "ALYA_REGISTRY_INDEX",
            format!("file://{}", base.display().to_string().replace('\\', "/")),
        );
        let hit = fetch_package_index("shardpkg");
        match prev {
            Some(v) => std::env::set_var("ALYA_REGISTRY_INDEX", v),
            None => std::env::remove_var("ALYA_REGISTRY_INDEX"),
        }
        let pkg = hit.expect("sharded file:// resolution");
        assert_eq!(pkg.versions[0].version, "2.0.0");
        let _ = std::fs::remove_dir_all(&base);
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

    #[test]
    fn cache_fresh_hit_and_stale_fallback() {
        // Same lock as the env-mutating index tests: a concurrent
        // `file://` window would reroute this fetch off the cache.
        let _env_guard = crate::tools::pkg::lock_registry_env();
        // Unique name: never collides with real packages or other tests.
        let name = format!("idxcachetest{}", std::process::id());
        let doc = format!(
            "{{\"name\": \"{}\", \"versions\": [{{\"version\": \"1.0.0\", \"tag\": \"v1.0.0\"}}]}}",
            name
        );
        let (json_path, meta_path) = index_cache_paths(&name).expect("cache dir");
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&json_path, &doc).unwrap();
        // Fresh (future timestamp): served without network.
        std::fs::write(
            &meta_path,
            format!("{{\"v\": 1, \"fetched_at\": {}}}", now_unix_secs() + 3600),
        )
        .unwrap();
        let hit = fetch_package_index(&name).expect("fresh cache hit");
        assert_eq!(hit.name, name);
        // Stale (old timestamp) with an unreachable origin: network fails,
        // stale fallback still serves.
        std::fs::write(&meta_path, "{\"v\": 1, \"fetched_at\": 1}").unwrap();
        let stale = fetch_package_index(&name).expect("stale fallback hit");
        assert_eq!(stale.name, name);
        let _ = std::fs::remove_file(&json_path);
        let _ = std::fs::remove_file(&meta_path);
    }
}
