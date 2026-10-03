use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkgCommand {
    Init {
        path: Option<String>,
        name: Option<String>,
        is_lib: bool,
    },
    Add {
        name: String,
        path: Option<String>,
        git: Option<String>,
        tag: Option<String>,
        branch: Option<String>,
        version: Option<String>,
        optional: bool,
    },
    Install {
        strict: bool,
        features: Vec<String>,
        no_default_features: bool,
        /// Workspace member selection (mirrors build/run/test flags).
        packages: Vec<String>,
        workspace: bool,
        exclude: Vec<String>,
    },
    List,
    Update {
        upgrade: bool,
        /// Accepted for uniformity with install; in a workspace update
        /// always spans all members (`--package`/`--exclude` are rejected,
        /// `--workspace` is a no-op).
        packages: Vec<String>,
        workspace: bool,
        exclude: Vec<String>,
    },
    Cache {
        clean: bool,
        all: bool,
    },
    Clean {
        all: bool,
    },
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub alya_version: Option<String>,
    pub links: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub entry: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub extra: BTreeMap<String, String>,
}

/// Edge-controlled feature requests on one dependency: `features` names
/// features of the dependency enabled whenever this edge is active, and
/// `default_features = false` opts that dependency out of its `default`
/// feature unless another active edge (or entry CLI) keeps it (additive:
/// one keeper wins). See spec Chapter 24 §1.5.1 and §1.7.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<String>,
}

impl DependencyEdge {
    pub fn plain() -> Self {
        Self {
            optional: false,
            default_features: true,
            features: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Version {
        version: String,
        edge: DependencyEdge,
    },
    Path {
        path: String,
        edge: DependencyEdge,
    },
    Git {
        url: String,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
        edge: DependencyEdge,
    },
}

/// One entry of a `[features]` member list (spec Chapter 24 §1.5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureMember {
    /// `name`: same-package feature and/or optional dependency.
    Local(String),
    /// `dep:name`: optional dependency, without touching same-named features.
    ExplicitDep(String),
    /// `name/feat`: feature `feat` of dependency `name` (enables `name`).
    DepFeature { dep: String, feature: String },
}

/// Structural split of a feature member. Returns `None` for empty parts or
/// more than one separator (`a/b/c`, `dep:`, …); character validation is the
/// manifest parser's job.
pub fn parse_feature_member(member: &str) -> Option<FeatureMember> {
    if member.is_empty() {
        return None;
    }
    if let Some(rest) = member.strip_prefix("dep:") {
        if rest.is_empty() || rest.contains([':', '/']) {
            return None;
        }
        return Some(FeatureMember::ExplicitDep(rest.to_string()));
    }
    if let Some((dep, feat)) = member.split_once('/') {
        if dep.is_empty() || feat.is_empty() || feat.contains([':', '/']) || dep.contains(':') {
            return None;
        }
        return Some(FeatureMember::DepFeature {
            dep: dep.to_string(),
            feature: feat.to_string(),
        });
    }
    if member.contains(':') {
        return None;
    }
    Some(FeatureMember::Local(member.to_string()))
}

impl DependencySource {
    /// Whether this dependency is opt-in via `[features]` (skipped by
    /// `install` unless an active feature enables it).
    pub fn is_optional(&self) -> bool {
        self.edge().optional
    }

    /// Shared edge view: optional flag, `default-features` opt-out, and
    /// the edge `features` request list.
    pub fn edge(&self) -> &DependencyEdge {
        match self {
            DependencySource::Version { edge, .. } => edge,
            DependencySource::Path { edge, .. } => edge,
            DependencySource::Git { edge, .. } => edge,
        }
    }

    /// Mutable edge view (resolution coalescing never touches the edge;
    /// `update` carries it over verbatim).
    pub fn edge_mut(&mut self) -> &mut DependencyEdge {
        match self {
            DependencySource::Version { edge, .. } => edge,
            DependencySource::Path { edge, .. } => edge,
            DependencySource::Git { edge, .. } => edge,
        }
    }
}

/// A single `[profile.<name>]` table. Closed key set by design: unknown
/// keys are rejected so typos fail loudly instead of silently doing nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildProfile {
    /// C/optimizer level 0-3. Does not change Alya codegen output (no codegen
    /// passes exist yet); feeds C compilation and link flags plus, later,
    /// incremental-cache keys.
    pub opt_level: u8,
    /// Debug info: `-g` for C objects, no `-s` at link. `false` strips.
    pub debug: bool,
    /// Link-time optimization (`-flto` at C compile and link; GCC/Clang).
    pub lto: bool,
}

impl BuildProfile {
    pub fn dev_default() -> Self {
        Self {
            opt_level: 0,
            debug: true,
            lto: false,
        }
    }

    pub fn release_default() -> Self {
        Self {
            opt_level: 3,
            debug: false,
            lto: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildConfig {
    pub links: Option<String>,
    pub c_sources: Vec<String>,
    pub c_flags: Vec<String>,
    pub c_include_dirs: Vec<String>,
    /// Platform-only C sources, compiled solely for the matching target OS
    /// (e.g. Win32 backends on Windows, Cocoa on macOS). Empty means none.
    pub c_sources_windows: Vec<String>,
    pub c_sources_macos: Vec<String>,
    pub c_sources_linux: Vec<String>,
    /// Link-only flags, passed to the final link step and never to C
    /// compiles: shared plus per-OS variants (`-lX11`, `-framework Cocoa`).
    pub c_link_flags: Vec<String>,
    pub c_link_flags_windows: Vec<String>,
    pub c_link_flags_macos: Vec<String>,
    pub c_link_flags_linux: Vec<String>,
    /// Unknown `[build]` keys preserved verbatim (`key = raw value`) so
    /// manifest rewrites never drop tool or user configuration.
    pub build_extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceConfig {
    /// Member declarations: literal relative paths (`libs/foo`) or
    /// single-level globs (`crates/*`). Relative to the workspace root.
    pub members: Vec<String>,
    /// Exclusions subtracted after member expansion (same syntax).
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub dependencies: BTreeMap<String, DependencySource>,
    pub build: Option<BuildConfig>,
    /// `[workspace]`: present on the workspace root. Virtual roots (no
    /// `[package]`) get a synthesized private package; member lists only.
    pub workspace: Option<WorkspaceConfig>,
    /// `[features]`: name -> member list. Members name a local feature, a
    /// (usually optional) dependency, `dep:name`, or `name/feat` (feature
    /// `feat` of dependency `name`, enabling it). Unified across the
    /// dependency graph (Chapter 24 §1.7.5); `@cfg(feature)` evaluates
    /// against each package's unified set (Chapter 18 §1.3).
    pub features: BTreeMap<String, Vec<String>>,
    /// `[profile.<name>]`: `dev`/`release` built in (defaults apply when the
    /// table is absent); any other name defines a custom profile selectable
    /// via `--profile <name>`.
    pub profiles: BTreeMap<String, BuildProfile>,
    /// Raw lines preserved from unrecognized sections (e.g. `[lint]`, `[fmt]`,
    /// `[test]`, `[bench]`) plus stray comment lines, keyed by section name
    /// (`""` holds top-of-file comments). Re-emitted verbatim on serialize
    /// so `add`/`update` never destroy tool configuration.
    pub section_extras: BTreeMap<String, Vec<String>>,
}

impl PackageManifest {
    pub fn links(&self) -> Option<&str> {
        self.build
            .as_ref()
            .and_then(|b| b.links.as_deref())
            .or(self.package.links.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub entry: String,
    pub checksum: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageLock {
    pub version: u32,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone)]
pub struct CachedPackageDetails {
    pub name: String,
    pub version: String,
    pub source: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
}
