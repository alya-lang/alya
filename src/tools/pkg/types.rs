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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Version {
        version: String,
        optional: bool,
    },
    Path {
        path: String,
        optional: bool,
    },
    Git {
        url: String,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
        optional: bool,
    },
}

impl DependencySource {
    /// Whether this dependency is opt-in via `[features]` (skipped by
    /// `install` unless an active feature enables it).
    pub fn is_optional(&self) -> bool {
        match self {
            DependencySource::Version { optional, .. } => *optional,
            DependencySource::Path { optional, .. } => *optional,
            DependencySource::Git { optional, .. } => *optional,
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
    /// `[features]`: name -> member list. Members name another feature or a
    /// (usually optional) dependency. No language-level `cfg(feature)` exists
    /// yet: features gate optional dependencies only.
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
