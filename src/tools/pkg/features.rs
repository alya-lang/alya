//! Feature and profile resolution for package manifests.
//!
//! Pure over parsed data (no I/O) so everything stays unit-testable:
//! - [`resolve_active_features`]: `default` (unless disabled) plus CLI extras,
//!   closed transitively over feature-to-feature references.
//! - [`enabled_dependencies`]: non-optional deps plus optionals named by an
//!   active feature.
//! - [`resolve_profile`]: built-in `dev`/`release` (with defaults when the
//!   table is absent) or a custom `[profile.<name>]`.
//! - [`profile_c_flags`] / [`profile_link_flags`]: flag contributions with
//!   explicit `[build]` flags always winning.
//!
//! Boundaries (v1, documented): no feature unification across the dependency
//! graph, no `dep/feature` propagation syntax, and no language-level
//! `cfg(feature)` — features gate optional dependencies only.

use super::discovery::find_manifest_dir_from;
use super::manifest::parse_manifest;
use super::types::{BuildProfile, PackageManifest};
use crate::codegen::{Architecture, OperatingSystem};
use crate::parser::CfgContext;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

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
    let mut active = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();
    if !no_default_features && manifest.features.contains_key("default") {
        stack.push("default".to_string());
    }
    stack.extend(cli_features.iter().cloned());
    while let Some(name) = stack.pop() {
        if !active.insert(name.clone()) {
            continue;
        }
        if let Some(members) = manifest.features.get(&name) {
            for member in members {
                if manifest.features.contains_key(member) {
                    stack.push(member.clone());
                }
            }
        }
    }
    Ok(active)
}

/// Dependency names the build may use: everything non-optional, plus
/// optionals named by a member of an active feature.
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
    for feature in active {
        if let Some(members) = manifest.features.get(feature) {
            for member in members {
                if manifest.dependencies.contains_key(member) {
                    enabled.insert(member.clone());
                }
            }
        }
    }
    enabled
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
        Architecture::X86 => "x86",
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
                optional: true,
            },
        );
        m.dependencies.insert(
            "tls".to_string(),
            DependencySource::Version {
                version: "0.1.0".to_string(),
                optional: true,
            },
        );
        m.dependencies.insert(
            "core".to_string(),
            DependencySource::Version {
                version: "0.1.0".to_string(),
                optional: false,
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
}
