use super::discovery::find_manifest_dir_from;
use super::manifest::{is_virtual_workspace_root, parse_manifest};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// A resolved workspace member: package name + directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    pub name: String,
    pub dir: PathBuf,
}

/// Walks up from `start` looking for an `alya.toml` that declares
/// `[workspace]`. Returns the root directory, if any.
pub fn find_workspace_root_from(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    // `start` may be a file (e.g. the input path); anchor at its parent.
    if current.is_file() {
        current.pop();
    }
    loop {
        let manifest = current.join("alya.toml");
        if manifest.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest) {
                if let Ok(parsed) = parse_manifest(&content) {
                    if parsed.workspace.is_some() {
                        return Some(current);
                    }
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Workspace root for the current process directory, if any.
pub fn find_workspace_root() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| find_workspace_root_from(&cwd))
}

/// Matches one path segment against `*`/`?` (no `/` crossing).
fn match_segment(pattern: &str, name: &str) -> bool {
    let (mut px, mut nx) = (pattern.as_bytes(), name.as_bytes());
    let mut star: Option<&[u8]> = None;
    let mut mark: &[u8] = nx;
    while !nx.is_empty() {
        if !px.is_empty() && (px[0] == b'?' || px[0] == nx[0]) {
            px = &px[1..];
            nx = &nx[1..];
        } else if !px.is_empty() && px[0] == b'*' {
            star = Some(&px[1..]);
            mark = nx;
            px = &px[1..];
        } else if let Some(star_px) = star {
            mark = &mark[1..];
            nx = mark;
            px = star_px;
        } else {
            return false;
        }
    }
    while !px.is_empty() && px[0] == b'*' {
        px = &px[1..];
    }
    px.is_empty()
}

/// Expands one member pattern (relative to `root`) into directories.
/// Only `*`/`?` within a segment are special; `**` is rejected at parse
/// time by the `..`-free rule and treated literally here.
fn expand_pattern(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let normalized = pattern.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Err(format!(
            "Invalid workspace member pattern '{}': empty path",
            pattern
        ));
    }
    let mut dirs = vec![root.to_path_buf()];
    for seg in segments {
        let mut next = Vec::new();
        if seg.contains('*') || seg.contains('?') {
            for base in &dirs {
                let entries = fs::read_dir(base).map_err(|e| {
                    format!(
                        "Cannot expand workspace pattern '{}' in '{}': {}",
                        pattern,
                        base.display(),
                        e
                    )
                })?;
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if match_segment(seg, &fname) {
                            next.push(entry.path());
                        }
                    }
                }
            }
        } else {
            for base in &dirs {
                let cand = base.join(seg);
                if cand.is_dir() {
                    next.push(cand);
                }
            }
        }
        dirs = next;
        if dirs.is_empty() {
            break;
        }
    }
    Ok(dirs)
}

/// Resolves all members of the workspace at `root`: expands `members`,
/// subtracts `exclude`, reads each member manifest. Errors on empty
/// expansion, missing manifests, nested workspaces, and duplicate names.
/// Members sort by package name for stable command output.
pub fn resolve_workspace_members(root: &Path) -> Result<Vec<WorkspaceMember>, String> {
    let content = fs::read_to_string(root.join("alya.toml"))
        .map_err(|e| format!("Failed to read workspace root manifest: {}", e))?;
    let manifest = parse_manifest(&content)?;
    let ws = manifest.workspace.as_ref().ok_or_else(|| {
        format!(
            "No [workspace] in '{}': not a workspace root",
            root.join("alya.toml").display()
        )
    })?;

    let mut included: BTreeSet<PathBuf> = BTreeSet::new();
    for pattern in &ws.members {
        let hits = expand_pattern(root, pattern)?;
        if hits.is_empty() {
            return Err(format!(
                "Workspace pattern '{}' in '{}' matched no directories",
                pattern,
                root.display()
            ));
        }
        for hit in hits {
            included.insert(hit);
        }
    }
    for pattern in &ws.exclude {
        for hit in expand_pattern(root, pattern)? {
            included.remove(&hit);
        }
    }
    if included.is_empty() {
        return Err(format!(
            "Workspace in '{}' resolves to no members (all excluded?)",
            root.display()
        ));
    }

    let mut members = Vec::new();
    let mut seen_names: BTreeSet<String> = BTreeSet::new();
    let mut dirs: Vec<PathBuf> = included.into_iter().collect();
    dirs.sort();
    for dir in dirs {
        let manifest_path = dir.join("alya.toml");
        if !manifest_path.is_file() {
            return Err(format!(
                "Workspace member '{}' has no alya.toml",
                dir.display()
            ));
        }
        let sub_content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read '{}': {}", manifest_path.display(), e))?;
        let sub = parse_manifest(&sub_content)?;
        if sub.workspace.is_some() {
            return Err(format!(
                "Nested workspace at '{}': members must not declare [workspace]",
                dir.display()
            ));
        }
        if is_virtual_workspace_root(&sub) {
            return Err(format!(
                "Workspace member '{}' must declare [package]",
                dir.display()
            ));
        }
        if !seen_names.insert(sub.package.name.clone()) {
            return Err(format!(
                "Duplicate workspace member name '{}' (at '{}')",
                sub.package.name,
                dir.display()
            ));
        }
        members.push(WorkspaceMember {
            name: sub.package.name,
            dir,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(members)
}

/// All buildable targets of a workspace: members plus the root package
/// itself when the root declares `[package]` (non-virtual). The root entry
/// lets root-payload workspaces install, update, document, and build the
/// root package instead of silently skipping it.
pub fn workspace_targets(root: &Path) -> Result<Vec<WorkspaceMember>, String> {
    let mut targets = resolve_workspace_members(root)?;
    let content = fs::read_to_string(root.join("alya.toml"))
        .map_err(|e| format!("Failed to read workspace root manifest: {}", e))?;
    let manifest = parse_manifest(&content)?;
    if !is_virtual_workspace_root(&manifest) {
        if targets.iter().any(|m| m.name == manifest.package.name) {
            return Err(format!(
                "Duplicate workspace member name '{}' (root package collides with a member)",
                manifest.package.name
            ));
        }
        targets.push(WorkspaceMember {
            name: manifest.package.name,
            dir: root.to_path_buf(),
        });
        targets.sort_by(|a, b| a.name.cmp(&b.name));
    }
    Ok(targets)
}

/// Selects target members for a command: `--package` filters by name,
/// `--exclude` subtracts, `--workspace`/root-cwd means all members.
/// `cwd_member` is the member containing the cwd (if any) for the
/// directory-sensitive default.
pub fn select_workspace_members(
    all: &[WorkspaceMember],
    packages: &[String],
    workspace: bool,
    exclude: &[String],
    cwd_member: Option<&Path>,
) -> Result<Vec<WorkspaceMember>, String> {
    if !packages.is_empty() && workspace {
        return Err("Error: '--package' cannot be combined with '--workspace'".to_string());
    }
    let excluded: BTreeSet<&str> = exclude.iter().map(|s| s.as_str()).collect();
    if !packages.is_empty() {
        let mut out = Vec::new();
        for want in packages {
            let found = all
                .iter()
                .find(|m| &m.name == want)
                .ok_or_else(|| format!("Error: workspace has no member named '{}'", want))?;
            if excluded.contains(found.name.as_str()) {
                return Err(format!(
                    "Error: member '{}' is both selected (--package) and excluded (--exclude)",
                    want
                ));
            }
            if !out.contains(found) {
                out.push(found.clone());
            }
        }
        return Ok(out);
    }
    if workspace {
        return Ok(all
            .iter()
            .filter(|m| !excluded.contains(m.name.as_str()))
            .cloned()
            .collect());
    }
    if !excluded.is_empty() {
        return Err("Error: '--exclude' requires '--workspace'".to_string());
    }
    // Directory-sensitive default: inside a member, that member alone.
    if let Some(cwd_dir) = cwd_member {
        if let Some(own) = all.iter().find(|m| m.dir == *cwd_dir) {
            return Ok(vec![own.clone()]);
        }
    }
    Ok(all.to_vec())
}

/// The member directory containing `start`, if `start` is inside a member
/// of the workspace at `root`.
pub fn member_containing(root: &Path, start: &Path) -> Option<PathBuf> {
    let members = resolve_workspace_members(root).ok()?;
    let anchor = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    let mut current = anchor;
    loop {
        if members.iter().any(|m| m.dir == current) {
            return Some(current);
        }
        if current == *root {
            return None;
        }
        if !current.pop() {
            return None;
        }
    }
}

/// True when `dir` (or its parents up to the filesystem root) is inside a
/// workspace — root itself or any member.
pub fn in_workspace_from(start: &Path) -> Option<PathBuf> {
    if let Some(root) = find_workspace_root_from(start) {
        return Some(root);
    }
    // A member dir without a root above it is not a workspace.
    let manifest_dir = find_manifest_dir_from(start)?;
    let content = fs::read_to_string(manifest_dir.join("alya.toml")).ok()?;
    let manifest = parse_manifest(&content).ok()?;
    if manifest.workspace.is_some() {
        return Some(manifest_dir);
    }
    None
}

/// Decides whether a build-like command fans out to workspace members.
///
/// Returns `Some(targets)` when the command must run once per member:
/// explicit selection flags, or a defaulted directory input (`alya test`,
/// or `alya build` with no file) whose cwd is the workspace root (or a
/// non-member dir). Returns `None` for single-package behavior, and errors
/// when selection flags are used outside a workspace.
pub fn resolve_command_targets(
    cwd: &Path,
    input_defaulted_to_dot: bool,
    packages: &[String],
    workspace: bool,
    exclude: &[String],
) -> Result<Option<Vec<WorkspaceMember>>, String> {
    let flags_used = !packages.is_empty() || workspace || !exclude.is_empty();
    let Some(root) = find_workspace_root_from(cwd) else {
        if flags_used {
            return Err(
                "Error: '--package'/'--workspace'/'--exclude' used outside a workspace (no [workspace] root found)"
                    .to_string(),
            );
        }
        return Ok(None);
    };
    if !flags_used {
        if !input_defaulted_to_dot {
            return Ok(None);
        }
        // Defaulted directory input: root (or stray dir) fans out to all
        // targets; inside a member it stays single-package.
        if member_containing(&root, cwd).is_some() {
            return Ok(None);
        }
        return Ok(Some(workspace_targets(&root)?));
    }
    let all = workspace_targets(&root)?;
    let cwd_member = member_containing(&root, cwd);
    Ok(Some(select_workspace_members(
        &all,
        packages,
        workspace,
        exclude,
        cwd_member.as_deref(),
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static WS_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_base(prefix: &str) -> PathBuf {
        let n = WS_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("alya_test_{}_{}_{}", prefix, std::process::id(), n))
    }

    fn write_manifest(dir: &Path, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("alya.toml"), body).unwrap();
    }

    fn member_manifest(name: &str) -> String {
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n",
            name
        )
    }

    fn scaffold_member(dir: &Path, name: &str, extra: &str) {
        write_manifest(dir, &format!("{}{}", member_manifest(name), extra));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src").join("main.alya"), "say \"hi\"\n").unwrap();
    }

    #[test]
    fn segment_matching() {
        assert!(match_segment("*", "anything"));
        assert!(match_segment("a*", "abc"));
        assert!(!match_segment("a*", "bac"));
        assert!(match_segment("a?c", "abc"));
        assert!(!match_segment("a?c", "ac"));
        assert!(match_segment("exact", "exact"));
        assert!(!match_segment("exact", "exact2"));
    }

    #[test]
    fn virtual_root_parses_and_validates() {
        let ok = parse_manifest("[workspace]\nmembers = [\"crates/*\"]\n").unwrap();
        assert!(ok.workspace.is_some());
        assert!(is_virtual_workspace_root(&ok));

        // Members-only: payload sections are rejected on virtual roots.
        let bad = "[workspace]\nmembers = [\"a\"]\n\n[dependencies]\nx = \"1.0.0\"\n";
        assert!(parse_manifest(bad).is_err());

        assert!(parse_manifest("[workspace]\nmembers = []\n").is_err());
        assert!(parse_manifest("[workspace]\nbogus = []\n").is_err());
        assert!(parse_manifest("[workspace]\nmembers = [\"../escape\"]\n").is_err());

        // Root package + members is allowed (root keeps its own payload).
        let both = parse_manifest(
            "[package]\nname = \"root\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\"a\"]\n",
        )
        .unwrap();
        assert!(both.workspace.is_some());
        assert!(!is_virtual_workspace_root(&both));
    }

    #[test]
    fn member_expansion_with_exclude() {
        let base = unique_base("wsexpand");
        let _ = fs::remove_dir_all(&base);
        write_manifest(
            &base,
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/skip\"]\n",
        );
        scaffold_member(&base.join("crates").join("a"), "a", "");
        scaffold_member(&base.join("crates").join("b"), "b", "");
        scaffold_member(&base.join("crates").join("skip"), "skip", "");

        let members = resolve_workspace_members(&base).unwrap();
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);

        // Glob matching nothing is a typo signal, not an empty workspace.
        write_manifest(&base, "[workspace]\nmembers = [\"empty/*\"]\n");
        assert!(resolve_workspace_members(&base).is_err());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_nested_workspaces_and_duplicates() {
        let base = unique_base("wsnested");
        let _ = fs::remove_dir_all(&base);
        write_manifest(&base, "[workspace]\nmembers = [\"a\", \"b\"]\n");
        // Member declaring its own workspace: no nesting.
        write_manifest(
            &base.join("a"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\"x\"]\n",
        );
        scaffold_member(&base.join("b"), "a", "");
        let err = resolve_workspace_members(&base).unwrap_err();
        assert!(err.contains("Nested workspace"), "got: {}", err);

        // Two members, one name.
        write_manifest(&base.join("a"), &member_manifest("dup"));
        write_manifest(&base.join("b"), &member_manifest("dup"));
        let err = resolve_workspace_members(&base).unwrap_err();
        assert!(err.contains("Duplicate"), "got: {}", err);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn selection_rules() {
        let all = vec![
            WorkspaceMember {
                name: "a".to_string(),
                dir: PathBuf::from("/ws/a"),
            },
            WorkspaceMember {
                name: "b".to_string(),
                dir: PathBuf::from("/ws/b"),
            },
        ];
        let one = select_workspace_members(&all, &["b".to_string()], false, &[], None).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].name, "b");
        assert!(select_workspace_members(&all, &["nope".to_string()], false, &[], None).is_err());
        assert!(select_workspace_members(&all, &["a".to_string()], true, &[], None).is_err());
        assert!(select_workspace_members(&all, &[], false, &["a".to_string()], None).is_err());
        let rest = select_workspace_members(&all, &[], true, &["a".to_string()], None).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].name, "b");
        // Directory default: inside member `a` selects `a` alone.
        let cwd =
            select_workspace_members(&all, &[], false, &[], Some(Path::new("/ws/a"))).unwrap();
        assert_eq!(cwd.len(), 1);
        assert_eq!(cwd[0].name, "a");
    }

    #[test]
    fn root_discovery_from_member() {
        let base = unique_base("wsroot");
        let _ = fs::remove_dir_all(&base);
        write_manifest(&base, "[workspace]\nmembers = [\"crates/*\"]\n");
        scaffold_member(&base.join("crates").join("a"), "a", "");
        let member_src = base.join("crates").join("a").join("src");

        assert_eq!(find_workspace_root_from(&member_src), Some(base.clone()));
        assert_eq!(find_workspace_root_from(&base), Some(base.clone()));
        assert_eq!(
            member_containing(&base, &member_src),
            Some(base.join("crates").join("a"))
        );
        assert_eq!(member_containing(&base, &base), None);

        let _ = fs::remove_dir_all(&base);
    }
}
