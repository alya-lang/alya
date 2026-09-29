//! Whole-program build cache: fingerprint-gated artifact reuse.
//!
//! Two-tier lookup around the expensive stages (codegen text emission,
//! assembly, GCC link):
//!
//! - Tier 1 (mtime journal): compares recorded mtimes/sizes/flags without
//!   reading any file bytes. Catches the unchanged rebuild.
//! - Tier 2 (content fingerprint): runs after the frontend (which discovers
//!   the input set) and catches touch-only changes.
//!
//! Miss → full build → store. `--fresh` forces rebuild + overwrite.
//!
//! Correctness rules (read before touching):
//! - The fingerprint covers every byte that can reach the output: all
//!   source files (main + transitive imports), the manifest, C sources for
//!   the target, effective C/link flags, codegen toggles, arch/os, command
//!   and output kinds, telling env vars, and the compiler identity itself
//!   (version + binary mtime/len, so dev builds never alias releases).
//! - Dependency *graphs* are not tracked: anything reachable only through
//!   content (extern libs derived from sources) is covered by source bytes.
//! - mtime comparison is equality-based (cargo-style): clock skew fails
//!   safe (perpetual miss), same-tick same-size edits are the accepted
//!   residual risk; Tier 2 content hashing still guards the build itself.
//! - Store is content-addressed under the global cache dir with a 1 GiB
//!   LRU cap (dir mtime = last use, refreshed on hit). `alya pkg clean`
//!   drops it alongside the other caches.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::tools::pkg::hash::sha256_hex;

/// LRU cap for the global build-artifact store.
pub const BUILD_CACHE_CAP_BYTES: u64 = 1024 * 1024 * 1024;

/// Store root: `~/.alya/cache/build`, or `.alya/cache/build` when no home
/// resolves. Entries are content-addressed, so projects safely share it.
pub fn build_cache_dir() -> PathBuf {
    crate::driver::toolchain::get_global_alya_dir()
        .map(|d| d.join("cache").join("build"))
        .unwrap_or_else(|| PathBuf::from(".alya").join("cache").join("build"))
}

/// Identity of the compiler binary itself. Dev builds share versions, so
/// the executable's mtime/len keeps them from aliasing each other.
pub fn compiler_id() -> String {
    let exe_sig = std::env::current_exe()
        .and_then(|p| std::fs::metadata(&p))
        .map(|m| {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("{}:{}", m.len(), mtime)
        })
        .unwrap_or_else(|_| "unknown".to_string());
    format!("{}:{}", env!("CARGO_PKG_VERSION"), exe_sig)
}

/// Env vars that change build outputs, when set.
pub fn watched_env() -> Vec<(String, String)> {
    [
        "ALYA_HOME",
        "ALYA_TOOLCHAIN_URL",
        "ALYA_TOOLCHAIN_AUTO_INSTALL",
    ]
    .iter()
    .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
    .collect()
}

/// Everything that determines one unit's output bytes.
pub struct FingerprintInputs {
    /// Canonical main-file path (forward slashes).
    pub main_path: String,
    /// (display path, bytes) for every source file incl. transitive imports.
    pub files: Vec<(String, Vec<u8>)>,
    /// Manifest bytes when inside a package.
    pub manifest_bytes: Option<Vec<u8>>,
    /// C source bytes compiled for this target.
    pub c_sources: Vec<(String, Vec<u8>)>,
    /// Auxiliary inputs (e.g. custom bundle icon bytes).
    pub aux_files: Vec<(String, Vec<u8>)>,
    /// Effective flags actually passed (profile already composed).
    pub c_flags: Vec<String>,
    pub link_flags: Vec<String>,
    /// Codegen toggles.
    pub no_std: bool,
    pub mem_trace: bool,
    pub arch: String,
    pub os: String,
    /// "run" | "build" | "test" | "bench".
    pub command: String,
    /// "exe" | "asm".
    pub output_kind: String,
    pub compiler_id: String,
    pub env: Vec<(String, String)>,
}

/// Caller-assembled raw inputs for one unit; [`build_query_raw`] turns
/// them into the canonical query (single code path for driver and test
/// runner, so lookup and store can never disagree).
pub struct RawQueryInputs {
    pub input_file: PathBuf,
    pub source: Vec<u8>,
    pub imports: Vec<PathBuf>,
    pub c_flags: Vec<String>,
    pub c_target_sources: Vec<PathBuf>,
    pub link_flags: Vec<String>,
    pub profile_name: String,
    pub features: Vec<String>,
    pub no_std: bool,
    pub mem_trace: bool,
    pub arch: String,
    pub os: String,
    pub command: String,
    pub output_kind: String,
    pub custom_icon: Option<PathBuf>,
}

/// All cache inputs for one unit, gathered once and reused for lookup
/// (both tiers) and store, so fingerprints never drift between the two.
pub struct BuildQuery {
    pub unit: UnitInputs,
    pub fingerprint: FingerprintInputs,
    pub journal_paths: Vec<PathBuf>,
}

/// Canonical query construction (sorting, canonicalization, reads).
/// Fails fast on unreadable inputs like a build would.
pub fn build_query_raw(raw: RawQueryInputs) -> Result<BuildQuery, String> {
    let input_canon =
        std::fs::canonicalize(&raw.input_file).unwrap_or_else(|_| raw.input_file.clone());
    let manifest_path = manifest_path_for(&input_canon);
    let manifest_dir = manifest_path
        .as_ref()
        .and_then(|p| p.parent().map(display_path))
        .unwrap_or_else(|| "none".to_string());
    let compiler = compiler_id();
    let unit = UnitInputs {
        main_path: display_path(&input_canon),
        manifest_dir,
        command: raw.command.clone(),
        arch: raw.arch.clone(),
        os: raw.os.clone(),
        profile: raw.profile_name.clone(),
        features: {
            let mut f = raw.features.clone();
            f.sort();
            f
        },
        compiler_id: compiler.clone(),
    };
    let mut import_paths = raw.imports;
    import_paths.sort();
    let mut import_files = read_file_bytes(&import_paths)?;
    let mut files = vec![(display_path(&input_canon), raw.source)];
    files.append(&mut import_files);
    let mut c_paths = raw.c_target_sources;
    c_paths.sort();
    let c_sources = read_file_bytes(&c_paths)?;
    let manifest_bytes = manifest_path.as_ref().and_then(|p| std::fs::read(p).ok());
    let mut journal_paths = vec![input_canon.clone()];
    journal_paths.extend(import_paths);
    if let Some(ref mp) = manifest_path {
        journal_paths.push(mp.clone());
    }
    journal_paths.extend(c_paths);
    let aux_files: Vec<(String, Vec<u8>)> = raw
        .custom_icon
        .as_ref()
        .and_then(|p| {
            std::fs::read(p)
                .ok()
                .map(|b| (format!("bundle-icon:{}", display_path(p)), b))
        })
        .into_iter()
        .collect();
    let fingerprint = FingerprintInputs {
        main_path: display_path(&input_canon),
        files,
        manifest_bytes,
        c_sources,
        aux_files,
        c_flags: raw.c_flags,
        link_flags: raw.link_flags,
        no_std: raw.no_std,
        mem_trace: raw.mem_trace,
        arch: raw.arch,
        os: raw.os,
        command: raw.command,
        output_kind: raw.output_kind,
        compiler_id: compiler,
        env: watched_env(),
    };
    Ok(BuildQuery {
        unit,
        fingerprint,
        journal_paths,
    })
}

fn feed(out: &mut Vec<u8>, tag: &str, val: &str) {
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(val.len().to_string().as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(val.as_bytes());
    out.extend_from_slice(b"\n");
}

fn feed_bytes(out: &mut Vec<u8>, tag: &str, val: &[u8]) {
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(val.len().to_string().as_bytes());
    out.extend_from_slice(b":");
    out.extend_from_slice(val);
    out.extend_from_slice(b"\n");
}

/// Canonical content fingerprint (hex). Field order and length prefixes
/// are part of the contract; changing them invalidates old entries
/// (safe direction: miss, never false hit).
pub fn fingerprint_hex(inputs: &FingerprintInputs) -> String {
    let mut out = Vec::new();
    feed(&mut out, "main", &inputs.main_path);
    let mut files = inputs.files.clone();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    out.extend_from_slice(format!("files:{}\n", files.len()).as_bytes());
    for (path, bytes) in &files {
        feed(&mut out, "path", path);
        feed_bytes(&mut out, "bytes", bytes);
    }
    match &inputs.manifest_bytes {
        Some(b) => feed_bytes(&mut out, "manifest", b),
        None => feed(&mut out, "manifest", "none"),
    }
    let mut c_sources = inputs.c_sources.clone();
    c_sources.sort_by(|a, b| a.0.cmp(&b.0));
    out.extend_from_slice(format!("csrc:{}\n", c_sources.len()).as_bytes());
    for (path, bytes) in &c_sources {
        feed(&mut out, "cpath", path);
        feed_bytes(&mut out, "cbytes", bytes);
    }
    let mut aux_files = inputs.aux_files.clone();
    aux_files.sort_by(|a, b| a.0.cmp(&b.0));
    out.extend_from_slice(format!("aux:{}\n", aux_files.len()).as_bytes());
    for (path, bytes) in &aux_files {
        feed(&mut out, "auxpath", path);
        feed_bytes(&mut out, "auxbytes", bytes);
    }
    for f in &inputs.c_flags {
        feed(&mut out, "cflag", f);
    }
    for f in &inputs.link_flags {
        feed(&mut out, "lflag", f);
    }
    feed(&mut out, "no_std", if inputs.no_std { "1" } else { "0" });
    feed(
        &mut out,
        "mem_trace",
        if inputs.mem_trace { "1" } else { "0" },
    );
    feed(&mut out, "arch", &inputs.arch);
    feed(&mut out, "os", &inputs.os);
    feed(&mut out, "command", &inputs.command);
    feed(&mut out, "output", &inputs.output_kind);
    feed(&mut out, "compiler", &inputs.compiler_id);
    let mut env = inputs.env.clone();
    env.sort();
    for (k, v) in &env {
        feed(&mut out, "env", &format!("{}={}", k, v));
    }
    sha256_hex(&out)
}

/// Identity of a cacheable unit (stable across runs, content-free).
pub struct UnitInputs {
    pub main_path: String,
    pub manifest_dir: String,
    pub command: String,
    pub arch: String,
    pub os: String,
    pub profile: String,
    pub features: Vec<String>,
    pub compiler_id: String,
}

pub fn unit_id_hex(unit: &UnitInputs) -> String {
    let mut out = Vec::new();
    feed(&mut out, "main", &unit.main_path);
    feed(&mut out, "manifest_dir", &unit.manifest_dir);
    feed(&mut out, "command", &unit.command);
    feed(&mut out, "arch", &unit.arch);
    feed(&mut out, "os", &unit.os);
    feed(&mut out, "profile", &unit.profile);
    let mut features = unit.features.clone();
    features.sort();
    for f in &features {
        feed(&mut out, "feature", f);
    }
    feed(&mut out, "compiler", &unit.compiler_id);
    sha256_hex(&out)
}

/// One Tier-1 record: mtime millis + len per file. Equality-compared.
pub struct JournalFile {
    pub path: String,
    pub mtime_ms: i64,
    pub len: u64,
}

pub fn file_journal_entry(path: &Path) -> Option<JournalFile> {
    let meta = fs::metadata(path).ok()?;
    let mtime_ms = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)?;
    Some(JournalFile {
        path: path.to_string_lossy().replace('\\', "/"),
        mtime_ms,
        len: meta.len(),
    })
}

fn journal_path(cache_dir: &Path, unit_hash: &str) -> PathBuf {
    cache_dir
        .join("journals")
        .join(format!("{}.json", unit_hash))
}

fn entry_dir(cache_dir: &Path, fingerprint: &str) -> PathBuf {
    cache_dir.join("entries").join(fingerprint)
}

pub fn artifact_path(cache_dir: &Path, fingerprint: &str) -> PathBuf {
    entry_dir(cache_dir, fingerprint).join("artifact")
}

/// A persisted Tier-1 record.
pub struct Journal {
    pub fingerprint: String,
    pub compiler: String,
    pub files: Vec<JournalFile>,
}

/// Reads a journal if present and well-formed. Corrupt journals are
/// treated as absent (delete + miss), never as hits.
pub fn read_journal(cache_dir: &Path, unit_hash: &str) -> Option<Journal> {
    let raw = fs::read_to_string(journal_path(cache_dir, unit_hash)).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    if v.get("v")?.as_u64()? != 1 {
        return None;
    }
    let mut files = Vec::new();
    for f in v.get("files")?.as_array()? {
        files.push(JournalFile {
            path: f.get("path")?.as_str()?.to_string(),
            mtime_ms: f.get("mtime_ms")?.as_i64()?,
            len: f.get("len")?.as_u64()?,
        });
    }
    Some(Journal {
        fingerprint: v.get("fingerprint")?.as_str()?.to_string(),
        compiler: v.get("compiler")?.as_str().unwrap_or("").to_string(),
        files,
    })
}

/// Tier 1: every recorded file still has the same mtime+len, the compiler
/// is unchanged, and the entry still exists. Reads directory metadata
/// only — no file bytes.
pub fn tier1_hit(cache_dir: &Path, journal: &Journal, compiler_id: &str) -> bool {
    if journal.compiler != compiler_id {
        return false;
    }
    for f in &journal.files {
        match file_journal_entry(Path::new(&f.path)) {
            Some(now) if now.mtime_ms == f.mtime_ms && now.len == f.len => {}
            _ => return false,
        }
    }
    artifact_path(cache_dir, &journal.fingerprint).is_file()
}

/// Persists entry artifact (atomically via temp+rename), journal, and
/// prunes the store to the cap. Returns the entry dir.
pub fn store(
    cache_dir: &Path,
    unit_hash: &str,
    fingerprint: &str,
    files: &[JournalFile],
    compiler_id: &str,
    artifact_bytes: &[u8],
) -> Result<PathBuf, String> {
    let dir = entry_dir(cache_dir, fingerprint);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Error: cannot create build cache entry: {}", e))?;
    let tmp = dir.join("artifact.tmp");
    fs::write(&tmp, artifact_bytes)
        .map_err(|e| format!("Error: cannot write build cache entry: {}", e))?;
    fs::rename(&tmp, dir.join("artifact"))
        .map_err(|e| format!("Error: cannot publish build cache entry: {}", e))?;
    store_journal(cache_dir, unit_hash, fingerprint, compiler_id, files)?;
    touch(&dir);
    prune_to_cap(cache_dir);
    Ok(dir)
}

/// Refreshes dir mtime for LRU accounting. Best effort.
pub fn touch(dir: &Path) {
    let tmp = dir.join(".touch");
    let _ = fs::write(&tmp, b"1");
    let _ = fs::remove_file(&tmp);
}

/// Deletes oldest-mtime entries (and their orphaned journals) until the
/// store fits the cap. Best effort; never fails the build.
pub fn prune_to_cap(cache_dir: &Path) {
    let entries = cache_dir.join("entries");
    let mut items: Vec<(i128, u64, PathBuf, String)> = Vec::new();
    let mut total: u64 = 0;
    if let Ok(rd) = fs::read_dir(&entries) {
        for entry in rd.flatten() {
            let path = entry.path();
            let fp = entry.file_name().to_string_lossy().to_string();
            let size = fs::read_dir(&path)
                .map(|rd| {
                    rd.flatten()
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .map(|m| m.len())
                        .sum::<u64>()
                })
                .unwrap_or(0);
            let mtime = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i128)
                .unwrap_or(0);
            total += size;
            items.push((mtime, size, path, fp));
        }
    }
    if total <= BUILD_CACHE_CAP_BYTES {
        return;
    }
    items.sort_by_key(|(mtime, _, _, _)| *mtime);
    for (_, size, path, _) in items {
        if total <= BUILD_CACHE_CAP_BYTES {
            break;
        }
        if fs::remove_dir_all(&path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
    // Drop journals whose entries are gone.
    if let Ok(rd) = fs::read_dir(cache_dir.join("journals")) {
        for entry in rd.flatten() {
            let path = entry.path();
            let Ok(raw) = fs::read_to_string(&path) else {
                continue;
            };
            let keep = serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v.get("fingerprint")?.as_str().map(|s| s.to_string()))
                .map(|fp| entry_dir(cache_dir, &fp).join("artifact").is_file())
                .unwrap_or(false);
            if !keep {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

/// Cache statistics for `alya pkg cache` and status reporting.
pub struct CacheStats {
    pub entries: usize,
    pub bytes: u64,
}

pub fn cache_stats(cache_dir: &Path) -> CacheStats {
    let mut stats = CacheStats {
        entries: 0,
        bytes: 0,
    };
    if let Ok(rd) = fs::read_dir(cache_dir.join("entries")) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            stats.entries += 1;
            if let Ok(rd) = fs::read_dir(&path) {
                for f in rd.flatten() {
                    if let Ok(m) = f.metadata() {
                        if m.is_file() && f.file_name().to_string_lossy() != ".touch" {
                            stats.bytes += m.len();
                        }
                    }
                }
            }
        }
    }
    stats
}

/// Restores the executable bit on a materialized cached executable.
/// Fresh links get it from the toolchain; cache copies must reapply it.
/// No-op on Windows.
pub fn mark_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Canonical display path (forward slashes) for fingerprint stability.
pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads (path, bytes) pairs for fingerprinting; missing files are an
/// error (fail safe: miss, never false hit).
pub fn read_file_bytes(paths: &[PathBuf]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let bytes = fs::read(p).map_err(|e| {
            format!(
                "Error: cannot read '{}' for fingerprint: {}",
                p.display(),
                e
            )
        })?;
        out.push((display_path(p), bytes));
    }
    Ok(out)
}

/// Builds journal records for Tier 1. Any unreadable file aborts the
/// journal (safe direction: no cache use).
pub fn journal_files(paths: &[PathBuf]) -> Option<Vec<JournalFile>> {
    paths.iter().map(|p| file_journal_entry(p)).collect()
}

pub fn manifest_path_for(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = dir.join("alya.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn manifest_bytes_for(path: &Path) -> Option<Vec<u8>> {
    let manifest = manifest_path_for(path)?;
    fs::read(manifest).ok()
}

/// A verified cache hit: artifact bytes plus how many files Tier 1/2
/// checked to establish it.
pub struct CacheHit {
    pub artifact: Vec<u8>,
    pub files_verified: usize,
}

/// Tier 1: journal mtimes/sizes + compiler identity, no content reads.
/// Returns the artifact on hit (refreshing LRU accounting).
pub fn tier1_lookup(cache_dir: &Path, unit: &UnitInputs) -> Option<CacheHit> {
    let unit_hash = unit_id_hex(unit);
    let journal = read_journal(cache_dir, &unit_hash)?;
    if !tier1_hit(cache_dir, &journal, &unit.compiler_id) {
        return None;
    }
    let artifact = fs::read(artifact_path(cache_dir, &journal.fingerprint)).ok()?;
    touch(&entry_dir(cache_dir, &journal.fingerprint));
    Some(CacheHit {
        artifact,
        files_verified: journal.files.len(),
    })
}

/// Tier 2: content fingerprint against the recorded one (catches
/// touch-only changes after the frontend ran). Rewrites the journal with
/// fresh mtimes on hit. Input read failures propagate (fail fast like a
/// build would); cache I/O failures degrade to miss.
pub fn tier2_lookup(
    cache_dir: &Path,
    unit: &UnitInputs,
    fp: &FingerprintInputs,
    journal_paths: &[PathBuf],
) -> Result<Option<CacheHit>, String> {
    let unit_hash = unit_id_hex(unit);
    let Some(journal) = read_journal(cache_dir, &unit_hash) else {
        return Ok(None);
    };
    if journal.compiler != unit.compiler_id {
        return Ok(None);
    }
    if fingerprint_hex(fp) != journal.fingerprint {
        return Ok(None);
    }
    let files = journal_files(journal_paths)
        .ok_or_else(|| "Error: cannot stat build inputs for cache journal".to_string())?;
    let entry = entry_dir(cache_dir, &journal.fingerprint);
    let artifact = fs::read(entry.join("artifact"))
        .map_err(|_| "Error: build cache entry vanished".to_string())?;
    // Refresh the journal (mtimes may have moved without content change).
    let _ = store_journal(
        cache_dir,
        &unit_hash,
        &journal.fingerprint,
        &unit.compiler_id,
        &files,
    );
    touch(&entry);
    Ok(Some(CacheHit {
        files_verified: files.len(),
        artifact,
    }))
}

fn store_journal(
    cache_dir: &Path,
    unit_hash: &str,
    fingerprint: &str,
    compiler_id: &str,
    files: &[JournalFile],
) -> Result<(), String> {
    let journal_files: Vec<Value> = files
        .iter()
        .map(|f| json!({"path": f.path, "mtime_ms": f.mtime_ms, "len": f.len}))
        .collect();
    let journal = json!({
        "v": 1,
        "fingerprint": fingerprint,
        "compiler": compiler_id,
        "files": journal_files,
    });
    let jdir = cache_dir.join("journals");
    fs::create_dir_all(&jdir)
        .map_err(|e| format!("Error: cannot create build cache journal: {}", e))?;
    fs::write(
        journal_path(cache_dir, unit_hash),
        serde_json::to_string(&journal).unwrap_or_default(),
    )
    .map_err(|e| format!("Error: cannot write build cache journal: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_inputs() -> FingerprintInputs {
        FingerprintInputs {
            main_path: "main.alya".to_string(),
            files: vec![("main.alya".to_string(), b"say 1\n".to_vec())],
            manifest_bytes: None,
            c_sources: Vec::new(),
            aux_files: Vec::new(),
            c_flags: vec!["-O2".to_string()],
            link_flags: Vec::new(),
            no_std: false,
            mem_trace: false,
            arch: "x64".to_string(),
            os: "windows".to_string(),
            command: "build".to_string(),
            output_kind: "exe".to_string(),
            compiler_id: "test-compiler".to_string(),
            env: Vec::new(),
        }
    }

    #[test]
    fn fingerprint_stable_and_sensitive() {
        let base = fingerprint_hex(&sample_inputs());
        assert_eq!(base, fingerprint_hex(&sample_inputs()));
        let mut changed = sample_inputs();
        changed.files[0].1 = b"say 2\n".to_vec();
        assert_ne!(base, fingerprint_hex(&changed));
        for mutate in [
            |i: &mut FingerprintInputs| i.arch = "arm64".to_string(),
            |i: &mut FingerprintInputs| i.command = "run".to_string(),
            |i: &mut FingerprintInputs| i.output_kind = "asm".to_string(),
            |i: &mut FingerprintInputs| i.compiler_id = "other".to_string(),
            |i: &mut FingerprintInputs| i.no_std = true,
            |i: &mut FingerprintInputs| i.mem_trace = true,
            |i: &mut FingerprintInputs| i.c_flags.push("-g".to_string()),
            |i: &mut FingerprintInputs| i.manifest_bytes = Some(b"[package]".to_vec()),
        ] {
            let mut other = sample_inputs();
            mutate(&mut other);
            assert_ne!(base, fingerprint_hex(&other));
        }
    }

    #[test]
    fn unit_id_separates_commands_and_profiles() {
        let unit = UnitInputs {
            main_path: "m".to_string(),
            manifest_dir: "d".to_string(),
            command: "build".to_string(),
            arch: "x64".to_string(),
            os: "windows".to_string(),
            profile: "dev".to_string(),
            features: Vec::new(),
            compiler_id: "c".to_string(),
        };
        let base = unit_id_hex(&unit);
        let mut run = unit;
        run.command = "run".to_string();
        assert_ne!(base, unit_id_hex(&run));
        run.command = "build".to_string();
        run.profile = "release".to_string();
        assert_ne!(base, unit_id_hex(&run));
    }

    #[test]
    fn tier2_catches_touch_only_changes() {
        // Same content, new mtime: Tier 1 misses, Tier 2 hits and refreshes
        // the journal so the next lookup is Tier 1 again.
        let dir = std::env::temp_dir().join(format!("alya_build_cache_t2_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let probe = dir.join("probe.txt");
        fs::write(&probe, b"same-bytes").unwrap();
        let unit = UnitInputs {
            main_path: "m".to_string(),
            manifest_dir: "none".to_string(),
            command: "build".to_string(),
            arch: "x64".to_string(),
            os: "windows".to_string(),
            profile: "dev".to_string(),
            features: Vec::new(),
            compiler_id: "comp".to_string(),
        };
        let fp = FingerprintInputs {
            main_path: "m".to_string(),
            files: vec![("m".to_string(), b"same-bytes".to_vec())],
            manifest_bytes: None,
            c_sources: Vec::new(),
            aux_files: Vec::new(),
            c_flags: Vec::new(),
            link_flags: Vec::new(),
            no_std: false,
            mem_trace: false,
            arch: "x64".to_string(),
            os: "windows".to_string(),
            command: "build".to_string(),
            output_kind: "exe".to_string(),
            compiler_id: "comp".to_string(),
            env: Vec::new(),
        };
        let paths = vec![probe.clone()];
        // Prime: store entry + journal.
        let files = journal_files(&paths).unwrap();
        let fp_hex = fingerprint_hex(&fp);
        let uh = unit_id_hex(&unit);
        store(&dir, &uh, &fp_hex, &files, "comp", b"exe").unwrap();
        // Touch (content identical, mtime moved).
        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::write(&probe, b"same-bytes").unwrap();
        // Tier 1 must miss (mtime moved)...
        let journal = read_journal(&dir, &uh).unwrap();
        assert!(!tier1_hit(&dir, &journal, "comp"));
        // ...but Tier 2 hits on content and refreshes the journal...
        let hit = tier2_lookup(&dir, &unit, &fp, &paths)
            .expect("tier2 io")
            .expect("tier2 hit");
        assert_eq!(hit.artifact, b"exe");
        // ...so Tier 1 passes again afterwards.
        let journal2 = read_journal(&dir, &uh).unwrap();
        assert!(tier1_hit(&dir, &journal2, "comp"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn journal_round_trip_and_tier1() {
        let dir =
            std::env::temp_dir().join(format!("alya_build_cache_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let probe = dir.join("probe.txt");
        fs::write(&probe, b"data").unwrap();
        let files = journal_files(std::slice::from_ref(&probe)).unwrap();
        let stored = store(&dir, "unit1", "fp1", &files, "comp", b"exe-bytes").unwrap();
        assert!(artifact_path(&dir, "fp1").is_file());
        let journal = read_journal(&dir, "unit1").expect("journal readable");
        assert_eq!(journal.fingerprint, "fp1");
        assert_eq!(journal.files.len(), 1);
        assert!(tier1_hit(&dir, &journal, "comp"));
        assert!(!tier1_hit(&dir, &journal, "other-compiler"));
        fs::write(&probe, b"changed!").unwrap();
        assert!(!tier1_hit(&dir, &journal, "comp"));
        let _ = fs::remove_dir_all(&dir);
        let _ = stored;
    }
}
