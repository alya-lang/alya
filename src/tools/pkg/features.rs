//! Feature and profile resolution for package manifests.
//!
//! Pure over parsed data (no I/O) so everything stays unit-testable:
//! - [`resolve_active_features`]: `default` (unless disabled) plus CLI extras,
//!   closed transitively over feature-to-feature references.
//! - [`enabled_dependencies`]: non-optional deps plus optionals named by an
//!   active feature (`name`, `dep:name`, or `name/feat`).
//! - [`unify_feature_sets`]: additive fixpoint over the dependency graph —
//!   each node accumulates its defaults (unless denied), its parents'
//!   `dep/feat` requests, and edge `features` lists (spec Chapter 24 §1.7.5).
//! - [`resolve_profile`]: built-in `dev`/`release` (with defaults when the
//!   table is absent) or a custom `[profile.<name>]`.
//! - [`profile_c_flags`] / [`profile_link_flags`]: flag contributions with
//!   explicit `[build]` flags always winning.
//!
//! `@cfg(feature)` evaluates against each package's unified set (spec
//! Chapter 18 §1.3). Boundaries (v1): no weak `dep?/feat` syntax, no
//! `package:feature` CLI selector.

use super::discovery::{find_manifest_dir_from, find_package_dir};
use super::manifest::parse_manifest;
use super::resolver::semver_major;
use super::types::{
    parse_feature_member, BuildProfile, DependencySource, FeatureMember, PackageManifest,
};
use crate::codegen::{Architecture, OperatingSystem};
use crate::parser::CfgContext;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves the active feature set. Unknown CLI names are hard errors
/// listing what's available. Terminates even on cyclic graphs (the manifest
/// parser rejects cycles, but the visited-set makes this total anyway).
pub fn resolve_active_features(
    manifest: &PackageManifest,
    cli_features: &[String],
    no_default_features: bool,
) -> Result<BTreeSet<String>, String> {
    for name in cli_features {
        if !manifest.features.contains_key(name) {
            let mut known: Vec<&str> = manifest.features.keys().map(|s| s.as_str()).collect();
            known.sort();
            return Err(format!(
                "Unknown feature '{}'. Available features: {}",
                name,
                if known.is_empty() {
                    "(none)".to_string()
                } else {
                    known.join(", ")
                }
            ));
        }
    }
    let mut seeds = BTreeSet::new();
    if !no_default_features && manifest.features.contains_key("default") {
        seeds.insert("default".to_string());
    }
    seeds.extend(cli_features.iter().cloned());
    Ok(close_features(manifest, &seeds))
}

/// Transitive local closure over `seeds`: follows `Local` members that name
/// another feature, and records `Local` members that name a dependency (Cargo
/// implicit-features parity: an enabled optional dependency is itself a
/// member of the active set, so `@cfg(feature = "<dep>")` sees it).
/// `dep:` / `dep/feat` members contribute cross-package requests, never
/// local names. Unknown seeds are dropped here: `install` already rejects
/// unknown foreign names against the target manifest, so anything reaching
/// this pure layer comes from the lenient compile-time path (uninstalled
/// leaves), where ignoring mirrors `@cfg(feature)` evaluating unknown names
/// to false — while the targeted dependency itself still activates via
/// [`enabled_dependencies`].
pub fn close_features(manifest: &PackageManifest, seeds: &BTreeSet<String>) -> BTreeSet<String> {
    let mut active = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();
    for seed in seeds {
        if manifest.features.contains_key(seed) || manifest.dependencies.contains_key(seed) {
            stack.push(seed.clone());
        }
    }
    while let Some(name) = stack.pop() {
        if !active.insert(name.clone()) {
            continue;
        }
        if let Some(members) = manifest.features.get(&name) {
            for member in members {
                if let Some(FeatureMember::Local(next)) = parse_feature_member(member) {
                    if manifest.features.contains_key(&next)
                        || manifest.dependencies.contains_key(&next)
                    {
                        stack.push(next);
                    }
                }
            }
        }
    }
    active
}

/// Dependency names the build may use: everything non-optional, plus
/// optionals switched on by the active set — named directly (`name` in an
/// active feature or in the seeds), explicitly (`dep:name`), or as a
/// propagation target (`name/feat`, which always activates `name`).
pub fn enabled_dependencies(
    manifest: &PackageManifest,
    active: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut enabled: BTreeSet<String> = manifest
        .dependencies
        .iter()
        .filter(|(_, dep)| !dep.is_optional())
        .map(|(name, _)| name.clone())
        .collect();
    // Seeds that directly name a dependency (requested features landing on
    // an optional dependency name, e.g. edge `features = ["tls"]`).
    for name in active {
        if manifest.dependencies.contains_key(name) {
            enabled.insert(name.clone());
        }
    }
    for feature in active {
        if let Some(members) = manifest.features.get(feature) {
            for member in members {
                match parse_feature_member(member) {
                    Some(FeatureMember::Local(name))
                        if manifest.dependencies.contains_key(&name) =>
                    {
                        enabled.insert(name);
                    }
                    Some(FeatureMember::ExplicitDep(dep)) => {
                        enabled.insert(dep);
                    }
                    Some(FeatureMember::DepFeature { dep, .. }) => {
                        enabled.insert(dep);
                    }
                    _ => {}
                }
            }
        }
    }
    enabled
}

/// Cross-package requests out of one closed feature set: `(dep, feat)` pairs
/// from `dep/feat` members. The caller additionally honors each active edge's
/// own `features` list from its [`DependencySource`].
pub fn dep_feature_requests(
    manifest: &PackageManifest,
    closed: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for feature in closed {
        if let Some(members) = manifest.features.get(feature) {
            for member in members {
                if let Some(FeatureMember::DepFeature { dep, feature }) =
                    parse_feature_member(member)
                {
                    out.push((dep, feature));
                }
            }
        }
    }
    out
}

/// Loader for [`unify_feature_sets`]: maps `(parent_id, dep_name, dep_source)`
/// to `(child_id, child_manifest)`, or `None` for unresolvable targets.
pub type ChildLoader<'a> =
    &'a mut dyn FnMut(&str, &str, &DependencySource) -> Option<(String, PackageManifest)>;

/// Additive feature unification over a dependency graph (spec Chapter 24
/// §1.7). Pure over caller-loaded manifests: `entries` are
/// `(id, manifest, seeds)` roots whose seeds are pre-decided (entry defaults
/// plus CLI — closure happens here).
///
/// Each non-entry node accumulates: its `default` feature unless *every*
/// recorded incoming edge sets `default-features = false`, plus every
/// `dep/feat` and edge-`features` request from any active parent. Incoming
/// records and seeds only grow, and nodes propagate only on growth, so the
/// worklist terminates (cross-package cycles converge).
pub fn unify_feature_sets(
    entries: Vec<(String, PackageManifest, BTreeSet<String>)>,
    load_child: ChildLoader<'_>,
) -> BTreeMap<String, BTreeSet<String>> {
    struct State {
        manifest: PackageManifest,
        seeds: BTreeSet<String>,
        closed: BTreeSet<String>,
        propagated: bool,
        is_entry: bool,
        /// (parent id, edge keeps defaults, edge features) per active edge.
        incoming: Vec<(String, bool, Vec<String>)>,
    }
    use std::collections::VecDeque;
    let mut nodes: BTreeMap<String, State> = BTreeMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for (id, manifest, seeds) in entries {
        nodes.insert(
            id.clone(),
            State {
                manifest,
                seeds,
                closed: BTreeSet::new(),
                propagated: false,
                is_entry: true,
                incoming: Vec::new(),
            },
        );
        queue.push_back(id);
    }
    while let Some(id) = queue.pop_front() {
        // Closed set under the currently recorded incoming edges.
        type NodeRequests = Vec<(String, DependencySource, Vec<String>)>;
        let (closed_new, requests): (BTreeSet<String>, NodeRequests) = {
            let node = match nodes.get(&id) {
                Some(n) => n,
                None => continue,
            };
            let mut seeds = node.seeds.clone();
            if !node.is_entry
                && node.manifest.features.contains_key("default")
                && (node.incoming.is_empty() || node.incoming.iter().any(|(_, keep, _)| *keep))
            {
                seeds.insert("default".to_string());
            }
            let closed = close_features(&node.manifest, &seeds);
            let enabled = enabled_dependencies(&node.manifest, &closed);
            let mut slash: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (dep, feat) in dep_feature_requests(&node.manifest, &closed) {
                slash.entry(dep).or_default().push(feat);
            }
            let mut reqs = Vec::new();
            for dep_name in &enabled {
                if let Some(dep) = node.manifest.dependencies.get(dep_name) {
                    let mut feats = slash.remove(dep_name).unwrap_or_default();
                    feats.extend(dep.edge().features.iter().cloned());
                    reqs.push((dep_name.clone(), dep.clone(), feats));
                }
            }
            (closed, reqs)
        };
        // Propagate only on growth (plus the mandatory first pass, which
        // carries edge `features` even for empty closures).
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
            let loaded = load_child(&id, &dep_name, &dep_source);
            let Some((child_id, child_manifest)) = loaded else {
                continue;
            };
            let keep = dep_source.edge().default_features;
            let edge_feats = dep_source.edge().features.clone();
            let is_new = !nodes.contains_key(&child_id);
            if is_new {
                nodes.insert(
                    child_id.clone(),
                    State {
                        manifest: child_manifest,
                        seeds: BTreeSet::new(),
                        closed: BTreeSet::new(),
                        propagated: false,
                        is_entry: false,
                        incoming: Vec::new(),
                    },
                );
            }
            let mut child_grew = is_new;
            if let Some(child) = nodes.get_mut(&child_id) {
                if !child.incoming.iter().any(|(p, _, _)| p == &id) {
                    // A new incoming edge re-opens the defaults question,
                    // so the child always re-evaluates.
                    child.incoming.push((id.clone(), keep, edge_feats));
                    child_grew = true;
                }
                // Unknown foreign names ride along inertly: `close_features`
                // drops them before evaluation, while the dep itself still
                // activates via `enabled_dependencies`.
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
    nodes.into_iter().map(|(id, n)| (id, n.closed)).collect()
}

/// Selects the build profile: a custom `[profile.<name>]` table wins, then
/// the built-in `dev`/`release` defaults. Unknown names list what's
/// available.
pub fn resolve_profile(manifest: &PackageManifest, name: &str) -> Result<BuildProfile, String> {
    if let Some(profile) = manifest.profiles.get(name) {
        return Ok(profile.clone());
    }
    match name {
        "dev" => Ok(BuildProfile::dev_default()),
        "release" => Ok(BuildProfile::release_default()),
        other => {
            let mut known = vec!["dev".to_string(), "release".to_string()];
            let mut custom: Vec<String> = manifest.profiles.keys().cloned().collect();
            custom.sort();
            known.extend(custom);
            Err(format!(
                "Unknown profile '{}'. Available profiles: {}",
                other,
                known.join(", ")
            ))
        }
    }
}

/// C-compile flags contributed by a profile, PREPENDED by the caller so
/// explicit `[build]` flags win (a later `-O` overrides an earlier one on
/// GCC/Clang; same for `-g`).
pub fn profile_c_flags(profile: &BuildProfile, existing: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if !existing.iter().any(|f| f.starts_with("-O")) {
        out.push(format!("-O{}", profile.opt_level));
    }
    if profile.debug && !existing.iter().any(|f| f.starts_with("-g")) {
        out.push("-g".to_string());
    }
    if profile.lto && !existing.iter().any(|f| f == "-flto") {
        out.push("-flto".to_string());
    }
    out
}

/// Link flags contributed by a profile (link-only, no conflicts possible).
/// Stripping is GNU-ld only: macOS uses no strip flag (Apple `ld` has no
/// `-s` equivalent; use the `strip` tool explicitly there).
pub fn profile_link_flags(profile: &BuildProfile, os: OperatingSystem) -> Vec<String> {
    let mut out = Vec::new();
    if profile.lto {
        out.push("-flto".to_string());
    }
    if !profile.debug && matches!(os, OperatingSystem::Windows | OperatingSystem::Linux) {
        out.push("-s".to_string());
    }
    out
}

/// Target OS name for `@cfg(os = ...)` evaluation.
pub fn target_os_name(os: OperatingSystem) -> &'static str {
    match os {
        OperatingSystem::Windows => "windows",
        OperatingSystem::Linux => "linux",
        OperatingSystem::MacOS => "macos",
    }
}

/// Target arch name for `@cfg(arch = ...)` evaluation (normalized; the
/// evaluator still tolerates `x86_64`/`aarch64` aliases).
pub fn target_arch_name(arch: Architecture) -> &'static str {
    match arch {
        Architecture::X64 => "x64",
        Architecture::ARM64 => "arm64",
    }
}

/// Builds the `@cfg` evaluation context for a compilation: target
/// os/arch (not the host — cross `--os`/`--arch` evaluate correctly),
/// the profile debug flag, and the active manifest features.
pub fn target_cfg(
    os: OperatingSystem,
    arch: Architecture,
    profile: &BuildProfile,
    features: &BTreeSet<String>,
) -> CfgContext {
    CfgContext::for_target(
        target_os_name(os),
        target_arch_name(arch),
        profile.debug,
        features,
    )
}

/// Canonicalizes a top manifest dir for same-package comparison.
/// `find_manifest_dir_from` yields `""` when `alya.toml` resolves against
/// the current directory (subdir entries like `examples/probe.alya`);
/// only `canonicalize(".")` equates that with the package root, otherwise
/// same-package imports wrongly fall back to dependency defaults
/// (alya-lang/alya#61). Unresolvable dirs keep their spelling (legacy).
fn canonical_top_dir(top: &Option<PathBuf>) -> Option<PathBuf> {
    top.as_ref().map(|d| {
        if d.as_os_str().is_empty() {
            std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."))
        } else {
            std::fs::canonicalize(d).unwrap_or_else(|_| d.clone())
        }
    })
}

/// Process-local memo of entry-rooted unification results, keyed by
/// canonical entry dir plus the sorted entry seeds. Manifest mtimes recorded
/// at computation time revalidate hits, so mid-process manifest rewrites
/// (tests, `add`) never read stale.
struct UnifiedCacheEntry {
    nodes: BTreeMap<String, BTreeSet<String>>,
    stamps: Vec<(PathBuf, u64, u64)>,
}
fn unified_cache() -> &'static std::sync::Mutex<BTreeMap<String, UnifiedCacheEntry>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, UnifiedCacheEntry>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
}

fn manifest_stamp(path: &Path) -> Option<(PathBuf, u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos() as u64;
    Some((path.to_path_buf(), mtime, meta.len()))
}

/// Entry-rooted unification over manifests on disk. `entry_dir` is the
/// canonical entry package dir, `entry_manifest` its parsed manifest, and
/// `seeds` the pre-decided entry set (entry defaults + CLI, i.e.
/// `inherit.features` at compile time). Non-path dependencies resolve exactly
/// like imports do ([`find_package_dir`] over the importing package's dir);
/// unresolvable targets become leaves. Results are memoized per process.
fn unified_nodes_for_entry(
    entry_dir: &Path,
    entry_manifest: PackageManifest,
    seeds: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let entry_id = entry_dir.to_string_lossy().replace('\\', "/");
    let mut seed_vec: Vec<&str> = seeds.iter().map(|s| s.as_str()).collect();
    seed_vec.sort();
    let key = format!("{}\0{}", entry_id, seed_vec.join(","));
    if let Ok(guard) = unified_cache().lock() {
        if let Some(hit) = guard.get(&key) {
            let fresh = hit.stamps.iter().all(|(path, mtime, len)| {
                manifest_stamp(path).is_some_and(|(_, m, l)| m == *mtime && l == *len)
            });
            if fresh {
                return hit.nodes.clone();
            }
        }
    }
    let mut stamps = Vec::new();
    if let Some(stamp) = manifest_stamp(&entry_dir.join("alya.toml")) {
        stamps.push(stamp);
    }
    let mut load_child = |parent_id: &str, dep_name: &str, dep: &DependencySource| {
        let parent_dir = Path::new(parent_id);
        let child_dir: PathBuf = match dep {
            DependencySource::Path { path, .. } => {
                let p = Path::new(path);
                let joined = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    parent_dir.join(p)
                };
                std::fs::canonicalize(&joined).unwrap_or(joined)
            }
            DependencySource::Git { tag, .. } => {
                let maj = tag.as_deref().and_then(semver_major);
                find_package_dir(parent_dir, dep_name, maj)
                    .unwrap_or_else(|| parent_dir.join(".alya").join("packages").join(dep_name))
            }
            DependencySource::Version { version, .. } => {
                let maj = semver_major(version);
                find_package_dir(parent_dir, dep_name, maj)
                    .unwrap_or_else(|| parent_dir.join(".alya").join("packages").join(dep_name))
            }
        };
        let manifest_path = child_dir.join("alya.toml");
        let content = fs::read_to_string(&manifest_path).ok()?;
        let manifest = parse_manifest(&content).ok()?;
        if let Some(stamp) = manifest_stamp(&manifest_path) {
            stamps.push(stamp);
        }
        let child_id = std::fs::canonicalize(&child_dir)
            .unwrap_or(child_dir)
            .to_string_lossy()
            .replace('\\', "/");
        Some((child_id, manifest))
    };
    let nodes = unify_feature_sets(
        vec![(entry_id, entry_manifest, seeds.clone())],
        &mut load_child,
    );
    stamps.sort();
    stamps.dedup();
    if let Ok(mut guard) = unified_cache().lock() {
        guard.insert(
            key,
            UnifiedCacheEntry {
                nodes: nodes.clone(),
                stamps,
            },
        );
        // Bounded memo: graphs are small but processes are long-lived (LSP).
        while guard.len() > 64 {
            let oldest = guard.keys().next().cloned();
            match oldest {
                Some(k) => {
                    guard.remove(&k);
                }
                None => break,
            }
        }
    }
    nodes
}

/// Canonical serialization of a unified feature map for cache keys: sorted
/// `id=feat,feat;…` pairs. Deterministic across runs on the same tree.
pub fn serialize_unified(nodes: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut out = String::from("unified:");
    for (id, feats) in nodes {
        out.push_str(id);
        out.push('=');
        out.push_str(&feats.iter().cloned().collect::<Vec<_>>().join(","));
        out.push(';');
    }
    out
}

/// Digest of the unified feature view for a compilation rooted at `start`
/// (file or dir) with pre-decided entry seeds. Feeds the incremental
/// build-cache fingerprint so feature flips rebuild affected nodes.
/// Never fails: unresolvable trees yield a stable `unresolved` marker.
pub fn unified_features_digest(start: &Path, entry_active: &BTreeSet<String>) -> String {
    let base: &Path = if start.is_file() {
        start.parent().unwrap_or(Path::new("."))
    } else {
        start
    };
    let Some(manifest_dir) = find_manifest_dir_from(base) else {
        return "unified:no-package".to_string();
    };
    let entry_dir = std::fs::canonicalize(&manifest_dir).unwrap_or(manifest_dir);
    let content = match fs::read_to_string(entry_dir.join("alya.toml")) {
        Ok(c) => c,
        Err(_) => return "unified:unresolved".to_string(),
    };
    let manifest = match parse_manifest(&content) {
        Ok(m) => m,
        Err(_) => return "unified:unresolved".to_string(),
    };
    serialize_unified(&unified_nodes_for_entry(&entry_dir, manifest, entry_active))
}

/// Config for parsing an IMPORTED file: its owning package's *unified*
/// feature set when it belongs to a different package than the entry, else
/// the inherited (top) context. Pseudo-paths (`<embedded:...>`) and files
/// outside any package always inherit. Packages unreachable from the entry
/// (e.g. not-installed leaves) fall back to their own defaults.
pub fn imported_file_cfg(
    file: &Path,
    top_manifest_dir: &Option<PathBuf>,
    inherit: &CfgContext,
) -> Result<CfgContext, String> {
    if !file.is_file() {
        return Ok(inherit.clone());
    }
    let base = file.parent().unwrap_or(Path::new("."));
    let Some(own_dir) = find_manifest_dir_from(base) else {
        return Ok(inherit.clone());
    };
    let own_dir = std::fs::canonicalize(&own_dir).unwrap_or(own_dir);
    match canonical_top_dir(top_manifest_dir) {
        Some(top_dir) if own_dir != top_dir => {
            let content = fs::read_to_string(top_dir.join("alya.toml"))
                .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
            let entry_manifest = parse_manifest(&content)?;
            let nodes = unified_nodes_for_entry(&top_dir, entry_manifest, &inherit.features);
            let own_id = own_dir.to_string_lossy().replace('\\', "/");
            match nodes.get(&own_id) {
                Some(active) => Ok(CfgContext {
                    features: active.clone(),
                    ..inherit.clone()
                }),
                None => {
                    let content = fs::read_to_string(own_dir.join("alya.toml"))
                        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
                    let manifest = parse_manifest(&content)?;
                    let active = resolve_active_features(&manifest, &[], false).unwrap_or_default();
                    Ok(CfgContext {
                        features: active,
                        ..inherit.clone()
                    })
                }
            }
        }
        _ => Ok(inherit.clone()),
    }
}

/// The resolved build configuration for one compilation: selected profile,
/// its values, and the validated active feature set.
#[derive(Debug, Clone)]
pub struct ResolvedBuild {
    pub profile_name: String,
    pub profile: BuildProfile,
    pub active_features: BTreeSet<String>,
}

/// Loads the package manifest enclosing `start` (a file or directory).
/// Returns `None` outside packages; single files build without one.
pub fn manifest_for_path(start: &Path) -> Result<Option<PackageManifest>, String> {
    let base: &Path = if start.is_file() {
        start.parent().unwrap_or(Path::new("."))
    } else {
        start
    };
    let Some(manifest_dir) = find_manifest_dir_from(base) else {
        return Ok(None);
    };
    let content = fs::read_to_string(manifest_dir.join("alya.toml"))
        .map_err(|e| format!("Failed to read alya.toml: {}", e))?;
    Ok(Some(parse_manifest(&content)?))
}

/// Resolves profile + features for a compilation rooted at `start`.
/// `--features` / `--no-default-features` require a package; `--profile`
/// falls back to the built-in `dev`/`release` shapes outside packages
/// (custom names need their `[profile.<name>]` table).
pub fn resolve_build_config(
    start: &Path,
    profile_name: &str,
    cli_features: &[String],
    no_default_features: bool,
) -> Result<ResolvedBuild, String> {
    match manifest_for_path(start)? {
        Some(manifest) => {
            let active = resolve_active_features(&manifest, cli_features, no_default_features)?;
            let profile = resolve_profile(&manifest, profile_name)?;
            Ok(ResolvedBuild {
                profile_name: profile_name.to_string(),
                profile,
                active_features: active,
            })
        }
        None => {
            if !cli_features.is_empty() || no_default_features {
                return Err(
                    "Error: '--features'/'--no-default-features' require a package (alya.toml)."
                        .to_string(),
                );
            }
            let profile = match profile_name {
                "dev" => BuildProfile::dev_default(),
                "release" => BuildProfile::release_default(),
                other => {
                    return Err(format!(
                        "Unknown profile '{}' outside a package. Available profiles: dev, release",
                        other
                    ));
                }
            };
            Ok(ResolvedBuild {
                profile_name: profile_name.to_string(),
                profile,
                active_features: BTreeSet::new(),
            })
        }
    }
}

pub fn manifest_for_tests() -> PackageManifest {
    use super::types::PackageInfo;
    PackageManifest {
        package: PackageInfo {
            name: "probe".to_string(),
            version: "0.1.0".to_string(),
            alya_version: None,
            links: None,
            authors: Vec::new(),
            description: None,
            entry: "src/main.alya".to_string(),
            license: None,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::pkg::types::DependencySource;

    fn manifest_with_features() -> PackageManifest {
        let mut m = manifest_for_tests();
        m.features
            .insert("default".to_string(), vec!["net".to_string()]);
        m.features.insert(
            "net".to_string(),
            vec!["http".to_string(), "tls".to_string()],
        );
        m.features
            .insert("simd".to_string(), vec!["net".to_string()]);
        m.dependencies.insert(
            "http".to_string(),
            DependencySource::Version {
                version: "0.1.0".to_string(),
                edge: crate::tools::pkg::types::DependencyEdge {
                    optional: true,
                    default_features: true,
                    features: Vec::new(),
                },
            },
        );
        m.dependencies.insert(
            "tls".to_string(),
            DependencySource::Version {
                version: "0.1.0".to_string(),
                edge: crate::tools::pkg::types::DependencyEdge {
                    optional: true,
                    default_features: true,
                    features: Vec::new(),
                },
            },
        );
        m.dependencies.insert(
            "core".to_string(),
            DependencySource::Version {
                version: "0.1.0".to_string(),
                edge: crate::tools::pkg::types::DependencyEdge::plain(),
            },
        );
        m
    }

    #[test]
    fn default_features_resolve_transitively() {
        let m = manifest_with_features();
        let active = resolve_active_features(&m, &[], false).unwrap();
        assert!(active.contains("default"));
        assert!(active.contains("net"));
        assert!(!active.contains("simd"));
        let enabled = enabled_dependencies(&m, &active);
        assert!(enabled.contains("core"));
        assert!(enabled.contains("http"));
        assert!(enabled.contains("tls"));
    }

    #[test]
    fn no_default_features_leaves_optionals_out() {
        let m = manifest_with_features();
        let active = resolve_active_features(&m, &[], true).unwrap();
        assert!(active.is_empty());
        let enabled = enabled_dependencies(&m, &active);
        assert_eq!(enabled, BTreeSet::from(["core".to_string()]));
    }

    #[test]
    fn cli_features_add_with_closure() {
        let m = manifest_with_features();
        let active = resolve_active_features(&m, &["simd".to_string()], true).unwrap();
        assert!(active.contains("simd"));
        assert!(active.contains("net"));
        let enabled = enabled_dependencies(&m, &active);
        assert!(enabled.contains("http"));
    }

    fn parse_manifest_for(s: &str) -> PackageManifest {
        crate::tools::pkg::manifest::parse_manifest(s).expect("test manifest must parse")
    }

    #[test]
    fn feature_member_shapes_parse() {
        assert_eq!(
            parse_feature_member("tls"),
            Some(FeatureMember::Local("tls".to_string()))
        );
        assert_eq!(
            parse_feature_member("dep:tls"),
            Some(FeatureMember::ExplicitDep("tls".to_string()))
        );
        assert_eq!(
            parse_feature_member("mid/tls"),
            Some(FeatureMember::DepFeature {
                dep: "mid".to_string(),
                feature: "tls".to_string(),
            })
        );
        assert_eq!(parse_feature_member(""), None);
        assert_eq!(parse_feature_member("dep:"), None);
        assert_eq!(parse_feature_member("a/b/c"), None);
        assert_eq!(parse_feature_member("mid/"), None);
        assert_eq!(parse_feature_member("/tls"), None);
        assert_eq!(parse_feature_member("dep:a/b"), None);
    }

    #[test]
    fn dep_colon_and_slash_enable_optionals() {
        let m = parse_manifest_for(
            "[package]\nname = \"x\"\n[dependencies]\n             tls = { version = \"0.1.0\", optional = true }\n             mid = { version = \"0.1.0\", optional = true }\n             [features]\nvia-dep = [\"dep:tls\"]\nvia-slash = [\"mid/aes\"]\n",
        );
        let active = close_features(
            &m,
            &BTreeSet::from(["via-dep".to_string(), "via-slash".to_string()]),
        );
        let enabled = enabled_dependencies(&m, &active);
        assert!(enabled.contains("tls"));
        assert!(enabled.contains("mid"));
        // Cross-package members never leak local names.
        assert!(!active.contains("aes"));
    }

    #[test]
    fn unify_propagates_dep_feat_chain() {
        let app = parse_manifest_for(
            "[package]\nname = \"app\"\n[dependencies]\nmid = { path = \"mid\" }\n[features]\nfull = [\"mid/tls\"]\n",
        );
        let mid = parse_manifest_for(
            "[package]\nname = \"mid\"\n[dependencies]\nleaf = { path = \"leaf\", optional = true }\n[features]\ntls = [\"leaf\"]\n",
        );
        let leaf = parse_manifest_for("[package]\nname = \"leaf\"\n");
        let mut table = BTreeMap::new();
        table.insert("mid".to_string(), mid);
        table.insert("leaf".to_string(), leaf);
        let app_active = resolve_active_features(&app, &["full".to_string()], true).unwrap();
        let nodes = unify_feature_sets(
            vec![("app".to_string(), app, app_active)],
            &mut |_parent: &str, dep: &str, _src: &DependencySource| {
                table.get(dep).cloned().map(|m| (dep.to_string(), m))
            },
        );
        assert!(nodes["mid"].contains("tls"));
        assert!(nodes["leaf"].is_empty() || !nodes["leaf"].contains("default"));
        // `leaf` was pulled in transitively through the propagated feature.
        assert!(nodes.contains_key("leaf"));
        // Entry keeps its explicit view.
        assert!(nodes["app"].contains("full"));
    }

    #[test]
    fn unify_defaults_denied_and_kept() {
        // All edges deny defaults -> dep default stays off.
        let app = parse_manifest_for(
            "[package]\nname = \"app\"\n[dependencies]\ndep = { path = \"dep\", default-features = false }\n",
        );
        let dep = parse_manifest_for(
            "[package]\nname = \"dep\"\n[features]\ndefault = [\"extra\"]\nextra = []\n",
        );
        let mut table = BTreeMap::new();
        table.insert("dep".to_string(), dep);
        let loader = &mut |_p: &str, d: &str, _s: &DependencySource| {
            table.get(d).cloned().map(|m| (d.to_string(), m))
        };
        let nodes = unify_feature_sets(vec![("app".to_string(), app, BTreeSet::new())], loader);
        assert!(!nodes["dep"].contains("default"));
        assert!(!nodes["dep"].contains("extra"));

        // One keeper edge is enough to keep defaults.
        let app2 = parse_manifest_for(
            "[package]\nname = \"app\"\n[dependencies]\ndep = { path = \"dep\" }\n",
        );
        let dep2 = parse_manifest_for(
            "[package]\nname = \"dep\"\n[features]\ndefault = [\"extra\"]\nextra = []\n",
        );
        let mut table2 = BTreeMap::new();
        table2.insert("dep".to_string(), dep2);
        let nodes2 = unify_feature_sets(
            vec![("app".to_string(), app2, BTreeSet::new())],
            &mut |_p: &str, d: &str, _s: &DependencySource| {
                table2.get(d).cloned().map(|m| (d.to_string(), m))
            },
        );
        assert!(nodes2["dep"].contains("default"));
        assert!(nodes2["dep"].contains("extra"));
    }

    #[test]
    fn unify_diamond_merges_requests() {
        let app = parse_manifest_for(
            "[package]\nname = \"app\"\n[dependencies]\nb = { path = \"b\" }\nc = { path = \"c\" }\n",
        );
        let b = parse_manifest_for(
            "[package]\nname = \"b\"\n[dependencies]\nz = { path = \"z\" }\n[features]\ndefault = [\"z/a\"]\n",
        );
        let c = parse_manifest_for(
            "[package]\nname = \"c\"\n[dependencies]\nz = { path = \"z\" }\n[features]\ndefault = [\"z/b\"]\n",
        );
        let z = parse_manifest_for("[package]\nname = \"z\"\n[features]\na = []\nb = []\n");
        let mut table = BTreeMap::new();
        table.insert("b".to_string(), b);
        table.insert("c".to_string(), c);
        table.insert("z".to_string(), z);
        let nodes = unify_feature_sets(
            vec![("app".to_string(), app, BTreeSet::new())],
            &mut |_p: &str, d: &str, _s: &DependencySource| {
                table.get(d).cloned().map(|m| (d.to_string(), m))
            },
        );
        // Both parents' requests land on the single shared node.
        assert!(nodes["z"].contains("a"));
        assert!(nodes["z"].contains("b"));
    }

    #[test]
    fn unify_cycle_terminates() {
        let a = parse_manifest_for(
            "[package]\nname = \"a\"\n[dependencies]\nb = { path = \"b\" }\n[features]\ndefault = [\"b/on\"]\nback = []\n",
        );
        let b = parse_manifest_for(
            "[package]\nname = \"b\"\n[dependencies]\na = { path = \"a\" }\n[features]\non = [\"a/back\"]\nback = []\n",
        );
        let mut table = BTreeMap::new();
        table.insert("a".to_string(), a.clone());
        table.insert("b".to_string(), b);
        let nodes = unify_feature_sets(
            vec![("a".to_string(), a, BTreeSet::from(["default".to_string()]))],
            &mut |_p: &str, d: &str, _s: &DependencySource| {
                table.get(d).cloned().map(|m| (d.to_string(), m))
            },
        );
        assert!(nodes["b"].contains("on"));
        assert!(nodes["a"].contains("back"));
    }

    #[test]
    fn unknown_cli_feature_errors() {
        let m = manifest_with_features();
        let err = resolve_active_features(&m, &["nope".to_string()], false).unwrap_err();
        assert!(err.contains("Unknown feature 'nope'"));
        assert!(err.contains("simd"));
    }

    #[test]
    fn profiles_resolve_with_defaults() {
        let m = manifest_for_tests();
        let dev = resolve_profile(&m, "dev").unwrap();
        assert_eq!(dev, BuildProfile::dev_default());
        let release = resolve_profile(&m, "release").unwrap();
        assert_eq!(release.opt_level, 3);
        assert!(!release.debug);
        let err = resolve_profile(&m, "nope").unwrap_err();
        assert!(err.contains("Unknown profile 'nope'"));
        assert!(err.contains("dev, release"));
    }

    #[test]
    fn custom_profile_overrides_builtin_shape() {
        let mut m = manifest_for_tests();
        m.profiles.insert(
            "tiny".to_string(),
            BuildProfile {
                opt_level: 1,
                debug: false,
                lto: true,
            },
        );
        let tiny = resolve_profile(&m, "tiny").unwrap();
        assert_eq!(tiny.opt_level, 1);
        assert!(tiny.lto);
    }

    #[test]
    fn profile_flags_prefer_explicit() {
        let release = BuildProfile::release_default();
        assert_eq!(profile_c_flags(&release, &[]), vec!["-O3".to_string()]);
        // Explicit -O2 wins; -g still added (debug release? no: release debug=false).
        assert_eq!(
            profile_c_flags(&release, &["-O2".to_string()]),
            Vec::<String>::new()
        );
        let dev = BuildProfile::dev_default();
        assert_eq!(
            profile_c_flags(&dev, &[]),
            vec!["-O0".to_string(), "-g".to_string()]
        );
        assert_eq!(
            profile_c_flags(&dev, &["-g0".to_string()]),
            vec!["-O0".to_string()]
        );
        // Link flags: release strips on GNU targets, never on macOS.
        assert_eq!(
            profile_link_flags(&release, OperatingSystem::Linux),
            vec!["-s".to_string()]
        );
        assert!(profile_link_flags(&release, OperatingSystem::MacOS).is_empty());
        assert!(profile_link_flags(&dev, OperatingSystem::Linux).is_empty());
    }

    #[test]
    fn canonical_top_dir_equates_empty_with_cwd() {
        // alya-lang/alya#61: subdir entries leave "" as the top manifest
        // dir; it must compare equal to the canonical package root.
        let cwd = std::fs::canonicalize(".").unwrap();
        assert_eq!(canonical_top_dir(&Some(PathBuf::new())), Some(cwd));
        assert_eq!(canonical_top_dir(&None), None);
        // Missing dirs keep their spelling (legacy fallback).
        let missing = PathBuf::from("does-not-exist-zzz");
        assert_eq!(canonical_top_dir(&Some(missing.clone())), Some(missing));
    }
}
