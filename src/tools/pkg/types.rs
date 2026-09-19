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
    },
    Install,
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
    Version(String),
    Path {
        path: String,
    },
    Git {
        url: String,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildConfig {
    pub links: Option<String>,
    pub c_sources: Vec<String>,
    pub c_flags: Vec<String>,
    pub c_include_dirs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub dependencies: BTreeMap<String, DependencySource>,
    pub build: Option<BuildConfig>,
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
