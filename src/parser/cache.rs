use crate::ast::Program;
use crate::parser::CfgContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory cache for module resolution and file AST parsing.
///
/// Designed to share parsed ASTs and canonical file paths across multiple
/// test suites during a single `alya test` or `alya bench` invocation.
#[derive(Debug, Default, Clone)]
pub struct ImportCache {
    /// Maps (current_dir, normalized_import_path) -> canonical_path
    pub resolved_paths: Arc<RwLock<HashMap<(PathBuf, String), PathBuf>>>,
    /// Maps (canonical_path, CfgContext) -> Program AST
    pub parsed_ast: Arc<RwLock<HashMap<(PathBuf, CfgContext), Program>>>,
}

impl ImportCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_resolved_path(&self, current_dir: &Path, import_path: &str) -> Option<PathBuf> {
        self.resolved_paths
            .read()
            .ok()?
            .get(&(current_dir.to_path_buf(), import_path.to_string()))
            .cloned()
    }

    pub fn insert_resolved_path(
        &self,
        current_dir: PathBuf,
        import_path: String,
        canonical: PathBuf,
    ) {
        if let Ok(mut map) = self.resolved_paths.write() {
            map.insert((current_dir, import_path), canonical);
        }
    }

    pub fn get_parsed_ast(&self, canonical: &Path, cfg: &CfgContext) -> Option<Program> {
        self.parsed_ast
            .read()
            .ok()?
            .get(&(canonical.to_path_buf(), cfg.clone()))
            .cloned()
    }

    pub fn insert_parsed_ast(&self, canonical: PathBuf, cfg: CfgContext, program: Program) {
        if let Ok(mut map) = self.parsed_ast.write() {
            map.insert((canonical, cfg), program);
        }
    }

    pub fn parsed_ast_count(&self) -> usize {
        self.parsed_ast.read().map(|m| m.len()).unwrap_or(0)
    }

    pub fn resolved_path_count(&self) -> usize {
        self.resolved_paths.read().map(|m| m.len()).unwrap_or(0)
    }
}
