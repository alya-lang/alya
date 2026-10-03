use super::toml::{parse_inline_table, parse_string_array, strip_toml_comment, unquote};
use super::types::{
    BuildConfig, BuildProfile, DependencyEdge, DependencySource, FeatureMember, PackageInfo,
    PackageManifest,
};
use std::collections::BTreeMap;

fn is_valid_feature_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Sections the manifest parser understands. Tool sections (`lint`, `fmt`,
/// `test`, `bench`) are preserved verbatim via `section_extras`; anything
/// else is a hard error so typos fail loudly instead of being ignored.
fn is_known_section(section: &str) -> bool {
    matches!(
        section,
        "" | "package"
            | "dependencies"
            | "build"
            | "features"
            | "workspace"
            | "lint"
            | "fmt"
            | "test"
            | "bench"
    ) || section.starts_with("profile.")
}

fn parse_profile_value(key: &str, val: &str, line_no: usize) -> Result<(String, String), String> {
    let err = |msg: &str| {
        format!(
            "Invalid [profile.*] entry '{}' in alya.toml at line {}: {}",
            key, line_no, msg
        )
    };
    match key {
        "opt-level" => {
            let level: u8 = val
                .parse()
                .map_err(|_| err("opt-level must be an integer 0-3"))?;
            if level > 3 {
                return Err(err("opt-level must be an integer 0-3"));
            }
            Ok((key.to_string(), level.to_string()))
        }
        "debug" | "lto" => {
            if val != "true" && val != "false" {
                return Err(err("expected 'true' or 'false'"));
            }
            Ok((key.to_string(), val.to_string()))
        }
        other => Err(format!(
            "Unknown [profile.*] key '{}' in alya.toml at line {}: expected one of 'opt-level', 'debug', 'lto'",
            other, line_no
        )),
    }
}

fn parse_optional_flag(
    table: &BTreeMap<String, String>,
    key: &str,
    line_no: usize,
) -> Result<bool, String> {
    match table.get("optional") {
        None => Ok(false),
        Some(v) if v == "true" => Ok(true),
        Some(v) if v == "false" => Ok(false),
        Some(v) => Err(format!(
            "Invalid dependency entry '{}' in alya.toml at line {}: 'optional' must be 'true' or 'false', got '{}'",
            key, line_no, v
        )),
    }
}

/// Parses the edge-control keys of one inline dependency table:
/// `optional` (default false), `default-features`/`default_features`
/// (default true), and `features` (a string array of dependency feature
/// names, default empty).
fn parse_dependency_edge(
    table: &BTreeMap<String, String>,
    key: &str,
    line_no: usize,
) -> Result<DependencyEdge, String> {
    let optional = parse_optional_flag(table, key, line_no)?;
    let default_features = match table
        .get("default-features")
        .or_else(|| table.get("default_features"))
    {
        None => true,
        Some(v) if v == "true" => true,
        Some(v) if v == "false" => false,
        Some(v) => {
            return Err(format!(
                "Invalid dependency entry '{}' in alya.toml at line {}: 'default-features' must be 'true' or 'false', got '{}'",
                key, line_no, v
            ));
        }
    };
    let features = match table.get("features") {
        None => Vec::new(),
        Some(raw) => {
            if !(raw.starts_with('[') && raw.ends_with(']')) {
                return Err(format!(
                    "Invalid dependency entry '{}' in alya.toml at line {}: 'features' must be a string array like '[\"tls\"]'",
                    key, line_no
                ));
            }
            let names = parse_string_array(raw);
            for name in &names {
                if !is_valid_feature_name(name) {
                    return Err(format!(
                        "Invalid feature '{}' in dependency entry '{}' in alya.toml at line {}",
                        name, key, line_no
                    ));
                }
            }
            names
        }
    };
    Ok(DependencyEdge {
        optional,
        default_features,
        features,
    })
}

/// Every feature member must name a local feature, a declared dependency,
/// `dep:name`, or `name/feat` (left side a declared dependency), and the
/// local feature-to-feature graph must be acyclic. Pure over parsed
/// tables so it stays unit-testable.
fn validate_feature_graph(
    features: &BTreeMap<String, Vec<String>>,
    dependencies: &BTreeMap<String, DependencySource>,
) -> Result<(), String> {
    for (feature, members) in features {
        for member in members {
            match super::types::parse_feature_member(member) {
                Some(FeatureMember::Local(name))
                    if is_valid_feature_name(&name)
                        && (features.contains_key(&name) || dependencies.contains_key(&name)) => {}
                Some(FeatureMember::ExplicitDep(dep))
                    if is_valid_feature_name(&dep) && dependencies.contains_key(&dep) => {}
                Some(FeatureMember::DepFeature { dep, feature: _ })
                    if is_valid_feature_name(&dep) && dependencies.contains_key(&dep) =>
                {
                    // Right side names a foreign feature: `install` rejects
                    // unknown names against the target manifest; the compiler
                    // stays lenient (uninstalled leaves fall back).
                }
                _ => {
                    return Err(format!(
                        "Feature '{}' in alya.toml references unknown feature or dependency '{}'",
                        feature, member
                    ));
                }
            }
        }
    }
    // Cycle detection (iterative DFS over feature->feature edges).
    // Self-edges (`feat = ["feat"]`, the idiomatic same-name optional-dep
    // form) are harmless no-ops: the traversal's visited-set already
    // terminates on them, so they are skipped here too.
    for root in features.keys() {
        let mut stack = vec![(root.clone(), false)];
        let mut visiting: Vec<String> = Vec::new();
        while let Some((node, expanded)) = stack.pop() {
            if expanded {
                visiting.retain(|n| n != &node);
                continue;
            }
            if visiting.contains(&node) {
                return Err(format!(
                    "Feature cycle detected in alya.toml involving '{}'",
                    node
                ));
            }
            visiting.push(node.clone());
            stack.push((node.clone(), true));
            if let Some(members) = features.get(&node) {
                for member in members {
                    if let Some(FeatureMember::Local(name)) =
                        super::types::parse_feature_member(member)
                    {
                        if name != *node && features.contains_key(&name) {
                            stack.push((name, false));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn build_profile_from_values(
    profile_name: &str,
    values: &BTreeMap<String, String>,
) -> Result<BuildProfile, String> {
    let is_release = profile_name == "release";
    let mut profile = if is_release {
        BuildProfile::release_default()
    } else {
        BuildProfile::dev_default()
    };
    for (key, val) in values {
        match key.as_str() {
            "opt-level" => {
                profile.opt_level = val.parse().map_err(|_| {
                    format!("Invalid opt-level '{}' in [profile.{}]", val, profile_name)
                })?;
            }
            "debug" => profile.debug = val == "true",
            "lto" => profile.lto = val == "true",
            other => {
                return Err(format!(
                    "Unknown [profile.{}] key '{}': expected one of 'opt-level', 'debug', 'lto'",
                    profile_name, other
                ));
            }
        }
    }
    Ok(profile)
}

fn is_section_header(trimmed: &str) -> Option<String> {
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        if !inner.is_empty()
            && inner
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        {
            return Some(inner.to_string());
        }
    }
    None
}

/// Collects verbatim lines the typed parser would otherwise drop: full
/// unknown sections plus stray `#` comment lines inside known sections and
/// at the top of the file. Keyed by section name (`""` = file top).
fn collect_section_extras(content: &str) -> BTreeMap<String, Vec<String>> {
    let mut extras: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current = String::new();
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = is_section_header(trimmed) {
            current = name;
            continue;
        }
        let known = matches!(
            current.as_str(),
            "" | "package" | "dependencies" | "build" | "features" | "workspace"
        ) || current.starts_with("profile.");
        if trimmed.starts_with('#') || !known {
            extras
                .entry(current.clone())
                .or_default()
                .push(trimmed.to_string());
        }
    }
    // Drop sections that ended up empty (e.g. headers with no body).
    extras.retain(|_, v| !v.is_empty());
    extras
}

pub fn parse_manifest(content: &str) -> Result<PackageManifest, String> {
    let mut name = String::new();
    let mut version = "0.1.0".to_string();
    let mut alya_version = None;
    let mut links = None;
    let mut build_links = None;
    let mut authors = Vec::new();
    let mut description = None;
    let mut entry = "src/main.alya".to_string();
    let mut license = None;
    let mut homepage = None;
    let mut repository = None;
    let mut keywords = Vec::new();
    let mut extra = BTreeMap::new();
    let mut dependencies = BTreeMap::new();
    let mut c_sources = Vec::new();
    let mut c_flags = Vec::new();
    let mut c_include_dirs = Vec::new();
    let mut c_sources_windows = Vec::new();
    let mut c_sources_macos = Vec::new();
    let mut c_sources_linux = Vec::new();
    let mut c_link_flags = Vec::new();
    let mut c_link_flags_windows = Vec::new();
    let mut c_link_flags_macos = Vec::new();
    let mut c_link_flags_linux = Vec::new();
    let mut build_extra = BTreeMap::new();
    let mut features: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut profile_values: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut workspace_members: Vec<String> = Vec::new();
    let mut workspace_exclude: Vec<String> = Vec::new();
    let mut workspace_seen = false;
    let section_extras = collect_section_extras(content);

    let mut current_section = String::new();

    let logical_lines = merge_multiline_toml(content);

    for (line_no, line_str) in logical_lines {
        let line = line_str.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            if !is_known_section(&current_section) {
                return Err(format!(
                    "Unknown section '[{}]' in alya.toml at line {}: expected one of '[package]', '[dependencies]', '[build]', '[features]', '[workspace]', '[profile.<name>]', '[lint]', '[fmt]', '[test]', '[bench]'",
                    current_section, line_no
                ));
            }
            continue;
        }

        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim();

            match current_section.as_str() {
                "package" => match key {
                    "name" => name = unquote(val),
                    "version" => version = unquote(val),
                    "alya-version" => alya_version = Some(unquote(val)),
                    "links" => links = Some(unquote(val)),
                    "authors" => authors = parse_string_array(val),
                    "description" => description = Some(unquote(val)),
                    "entry" => entry = unquote(val),
                    "license" => license = Some(unquote(val)),
                    "homepage" => homepage = Some(unquote(val)),
                    "repository" => repository = Some(unquote(val)),
                    "keywords" => keywords = parse_string_array(val),
                    other => {
                        extra.insert(other.to_string(), val.to_string());
                    }
                },
                "build" => match key {
                    "links" => build_links = Some(unquote(val)),
                    "c-sources" | "c_sources" => c_sources = parse_string_array(val),
                    "c-sources-windows" | "c_sources_windows" => {
                        c_sources_windows = parse_string_array(val)
                    }
                    "c-sources-macos" | "c_sources_macos" => {
                        c_sources_macos = parse_string_array(val)
                    }
                    "c-sources-linux" | "c_sources_linux" => {
                        c_sources_linux = parse_string_array(val)
                    }
                    "c-link-flags" | "c_link_flags" => c_link_flags = parse_string_array(val),
                    "c-link-flags-windows" | "c_link_flags_windows" => {
                        c_link_flags_windows = parse_string_array(val)
                    }
                    "c-link-flags-macos" | "c_link_flags_macos" => {
                        c_link_flags_macos = parse_string_array(val)
                    }
                    "c-link-flags-linux" | "c_link_flags_linux" => {
                        c_link_flags_linux = parse_string_array(val)
                    }
                    "c-flags" | "c_flags" => c_flags = parse_string_array(val),
                    "c-include-dirs" | "c_include_dirs" => c_include_dirs = parse_string_array(val),
                    other => {
                        build_extra.insert(other.to_string(), val.to_string());
                    }
                },
                "dependencies" => {
                    if val.starts_with('{') {
                        let table = parse_inline_table(val);
                        let edge = parse_dependency_edge(&table, key, line_no)?;
                        if let Some(p) = table.get("path") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Path {
                                    path: p.clone(),
                                    edge,
                                },
                            );
                        } else if let Some(g) = table.get("git") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Git {
                                    url: g.clone(),
                                    tag: table.get("tag").cloned(),
                                    branch: table.get("branch").cloned(),
                                    rev: table.get("rev").cloned(),
                                    edge,
                                },
                            );
                        } else if let Some(v_inner) = table.get("version") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Version {
                                    version: v_inner.clone(),
                                    edge,
                                },
                            );
                        } else {
                            return Err(format!(
                                "Invalid dependency entry '{}' in alya.toml at line {}: inline table must declare one of 'path', 'git', or 'version'",
                                key, line_no
                            ));
                        }
                    } else {
                        dependencies.insert(
                            key.to_string(),
                            DependencySource::Version {
                                version: unquote(val),
                                edge: DependencyEdge::plain(),
                            },
                        );
                    }
                }
                "features" => {
                    if !is_valid_feature_name(key) {
                        return Err(format!(
                            "Invalid feature name '{}' in alya.toml at line {}: expected alphanumeric, '_' or '-'",
                            key, line_no
                        ));
                    }
                    if !(val.starts_with('[') && val.ends_with(']')) {
                        return Err(format!(
                            "Invalid feature '{}' in alya.toml at line {}: value must be a string array like '[\"dep\", \"other-feature\"]'",
                            key, line_no
                        ));
                    }
                    let members = parse_string_array(val);
                    for member in &members {
                        let ok = match super::types::parse_feature_member(member) {
                            Some(FeatureMember::Local(name)) => is_valid_feature_name(&name),
                            Some(FeatureMember::ExplicitDep(dep)) => is_valid_feature_name(&dep),
                            Some(FeatureMember::DepFeature { dep, feature }) => {
                                is_valid_feature_name(&dep) && is_valid_feature_name(&feature)
                            }
                            None => false,
                        };
                        if !ok {
                            return Err(format!(
                                "Invalid feature member '{}' in feature '{}' in alya.toml at line {}: expected 'name', 'dep:name', or 'name/feat'",
                                member, key, line_no
                            ));
                        }
                    }
                    features.insert(key.to_string(), members);
                }
                "workspace" => {
                    workspace_seen = true;
                    match key {
                        "members" | "exclude" => {
                            if !(val.starts_with('[') && val.ends_with(']')) {
                                return Err(format!(
                                    "Invalid [workspace] '{}' in alya.toml at line {}: value must be a string array like '[\"crates/*\"]'",
                                    key, line_no
                                ));
                            }
                            let entries = parse_string_array(val);
                            for entry in &entries {
                                if entry.is_empty()
                                    || entry.starts_with('/')
                                    || entry.starts_with('\\')
                                    || entry.contains("..")
                                {
                                    return Err(format!(
                                        "Invalid [workspace] '{}' entry '{}' in alya.toml at line {}: expected a relative path without '..'",
                                        key, entry, line_no
                                    ));
                                }
                            }
                            if key == "members" {
                                workspace_members = entries;
                            } else {
                                workspace_exclude = entries;
                            }
                        }
                        other => {
                            return Err(format!(
                                "Unknown [workspace] key '{}' in alya.toml at line {}: expected one of 'members', 'exclude'",
                                other, line_no
                            ));
                        }
                    }
                }
                _ => {}
            }
            if let Some(profile_name) = current_section.strip_prefix("profile.") {
                if !is_valid_feature_name(profile_name) {
                    return Err(format!(
                        "Invalid profile name '{}' in alya.toml at line {}",
                        profile_name, line_no
                    ));
                }
                let (k, v) = parse_profile_value(key, val, line_no)?;
                profile_values
                    .entry(profile_name.to_string())
                    .or_default()
                    .insert(k, v);
            }
        } else {
            return Err(format!(
                "Syntax error in alya.toml at line {}: '{}'",
                line_no, line
            ));
        }
    }

    let workspace = if workspace_seen {
        if workspace_members.is_empty() {
            return Err(
                "Invalid [workspace] in alya.toml: 'members' must list at least one member"
                    .to_string(),
            );
        }
        Some(crate::tools::pkg::types::WorkspaceConfig {
            members: workspace_members,
            exclude: workspace_exclude,
        })
    } else {
        None
    };

    // Virtual roots declare members only: no package payload of their own.
    let is_virtual = name.is_empty();
    if is_virtual && workspace.is_none() {
        return Err("Missing required field 'name' under [package] in alya.toml".to_string());
    }
    if is_virtual {
        if !dependencies.is_empty()
            || !features.is_empty()
            || !profile_values.is_empty()
            || build_links.is_some()
            || !c_sources.is_empty()
            || !build_extra.is_empty()
        {
            return Err(
                "Invalid virtual [workspace] root in alya.toml: a root without [package] must not declare [dependencies], [features], [profile.*] or [build]"
                    .to_string(),
            );
        }
        name = "__workspace_root__".to_string();
    }

    validate_feature_graph(&features, &dependencies)?;

    let mut profiles: BTreeMap<String, BuildProfile> = BTreeMap::new();
    for (profile_name, values) in &profile_values {
        profiles.insert(
            profile_name.clone(),
            build_profile_from_values(profile_name, values)?,
        );
    }

    let build = if !c_sources.is_empty()
        || !c_flags.is_empty()
        || !c_include_dirs.is_empty()
        || !c_sources_windows.is_empty()
        || !c_sources_macos.is_empty()
        || !c_sources_linux.is_empty()
        || !c_link_flags.is_empty()
        || !c_link_flags_windows.is_empty()
        || !c_link_flags_macos.is_empty()
        || !c_link_flags_linux.is_empty()
        || build_links.is_some()
        || !build_extra.is_empty()
    {
        Some(BuildConfig {
            links: build_links,
            c_sources,
            c_flags,
            c_include_dirs,
            c_sources_windows,
            c_sources_macos,
            c_sources_linux,
            c_link_flags,
            c_link_flags_windows,
            c_link_flags_macos,
            c_link_flags_linux,
            build_extra,
        })
    } else {
        None
    };

    Ok(PackageManifest {
        package: PackageInfo {
            name,
            version,
            alya_version,
            links,
            authors,
            description,
            entry,
            license,
            homepage,
            repository,
            keywords,
            extra,
        },
        dependencies,
        build,
        features,
        profiles,
        workspace,
        section_extras,
    })
}

pub fn is_virtual_workspace_root(manifest: &PackageManifest) -> bool {
    // Synthesized private package marks a members-only root.
    manifest.workspace.is_some() && manifest.package.name == "__workspace_root__"
}

pub fn serialize_manifest(manifest: &PackageManifest) -> String {
    let mut out = String::new();
    if let Some(top) = manifest.section_extras.get("") {
        for line in top {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str("[package]\n");
    out.push_str(&format!("name = \"{}\"\n", manifest.package.name));
    out.push_str(&format!("version = \"{}\"\n", manifest.package.version));
    if let Some(av) = &manifest.package.alya_version {
        out.push_str(&format!("alya-version = \"{}\"\n", av));
    }
    if let Some(l) = &manifest.package.links {
        out.push_str(&format!("links = \"{}\"\n", l));
    }
    out.push_str(&format!("entry = \"{}\"\n", manifest.package.entry));
    if let Some(desc) = &manifest.package.description {
        out.push_str(&format!("description = \"{}\"\n", desc));
    }
    if !manifest.package.authors.is_empty() {
        let authors_str = manifest
            .package
            .authors
            .iter()
            .map(|a| format!("\"{}\"", a))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("authors = [{}]\n", authors_str));
    }
    if let Some(lic) = &manifest.package.license {
        out.push_str(&format!("license = \"{}\"\n", lic));
    }
    if let Some(home) = &manifest.package.homepage {
        out.push_str(&format!("homepage = \"{}\"\n", home));
    }
    if let Some(repo) = &manifest.package.repository {
        out.push_str(&format!("repository = \"{}\"\n", repo));
    }
    if !manifest.package.keywords.is_empty() {
        let keywords_str = manifest
            .package
            .keywords
            .iter()
            .map(|k| format!("\"{}\"", k))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("keywords = [{}]\n", keywords_str));
    }
    for (k, v) in &manifest.package.extra {
        out.push_str(&format!("{} = {}\n", k, v));
    }
    if let Some(pkg_extras) = manifest.section_extras.get("package") {
        for line in pkg_extras {
            out.push_str(line);
            out.push('\n');
        }
    }

    if let Some(ws) = &manifest.workspace {
        out.push_str("\n[workspace]\n");
        let members_str = ws
            .members
            .iter()
            .map(|m| format!("\"{}\"", m))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("members = [{}]\n", members_str));
        if !ws.exclude.is_empty() {
            let exclude_str = ws
                .exclude
                .iter()
                .map(|m| format!("\"{}\"", m))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("exclude = [{}]\n", exclude_str));
        }
        if let Some(ws_extras) = manifest.section_extras.get("workspace") {
            for line in ws_extras {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out.push_str("\n[dependencies]\n");
    for (name, dep) in &manifest.dependencies {
        // Edge-control suffix shared by every source kind: empty for plain
        // edges so round-trips of legacy manifests stay byte-stable.
        let edge_suffix = |edge: &DependencyEdge| -> String {
            let mut s = String::new();
            if edge.optional {
                s.push_str(", optional = true");
            }
            if !edge.default_features {
                s.push_str(", default-features = false");
            }
            if !edge.features.is_empty() {
                let list = edge
                    .features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ");
                s.push_str(&format!(", features = [{}]", list));
            }
            s
        };
        match dep {
            DependencySource::Version { version, edge } => {
                let suffix = edge_suffix(edge);
                if suffix.is_empty() {
                    out.push_str(&format!("{} = \"{}\"\n", name, version));
                } else {
                    out.push_str(&format!(
                        "{} = {{ version = \"{}\"{} }}\n",
                        name, version, suffix
                    ));
                }
            }
            DependencySource::Path { path, edge } => {
                out.push_str(&format!(
                    "{} = {{ path = \"{}\"{} }}\n",
                    name,
                    path.replace('\\', "/"),
                    edge_suffix(edge)
                ));
            }
            DependencySource::Git {
                url,
                tag,
                branch,
                rev,
                edge,
            } => {
                let mut parts = vec![format!("git = \"{}\"", url)];
                if let Some(t) = tag {
                    parts.push(format!("tag = \"{}\"", t));
                }
                if let Some(b) = branch {
                    parts.push(format!("branch = \"{}\"", b));
                }
                if let Some(r) = rev {
                    parts.push(format!("rev = \"{}\"", r));
                }
                let suffix = edge_suffix(edge);
                if !suffix.is_empty() {
                    parts.push(suffix.trim_start_matches(", ").to_string());
                }
                out.push_str(&format!("{} = {{ {} }}\n", name, parts.join(", ")));
            }
        }
    }
    if let Some(dep_extras) = manifest.section_extras.get("dependencies") {
        for line in dep_extras {
            out.push_str(line);
            out.push('\n');
        }
    }

    if let Some(b) = &manifest.build {
        out.push_str("\n[build]\n");
        if let Some(l) = &b.links {
            out.push_str(&format!("links = \"{}\"\n", l));
        }
        if !b.c_sources.is_empty() {
            let sources_str = b
                .c_sources
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("c-sources = [{}]\n", sources_str));
        }
        if !b.c_flags.is_empty() {
            let flags_str = b
                .c_flags
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("c-flags = [{}]\n", flags_str));
        }
        if !b.c_include_dirs.is_empty() {
            let inc_str = b
                .c_include_dirs
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("c-include-dirs = [{}]\n", inc_str));
        }
        for (key, sources) in [
            ("c-sources-windows", &b.c_sources_windows),
            ("c-sources-macos", &b.c_sources_macos),
            ("c-sources-linux", &b.c_sources_linux),
            ("c-link-flags", &b.c_link_flags),
            ("c-link-flags-windows", &b.c_link_flags_windows),
            ("c-link-flags-macos", &b.c_link_flags_macos),
            ("c-link-flags-linux", &b.c_link_flags_linux),
        ] {
            if !sources.is_empty() {
                let sources_str = sources
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("{} = [{}]\n", key, sources_str));
            }
        }
        for (k, v) in &b.build_extra {
            out.push_str(&format!("{} = {}\n", k, v));
        }
        if let Some(build_extras) = manifest.section_extras.get("build") {
            for line in build_extras {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    if !manifest.features.is_empty() {
        out.push_str("\n[features]\n");
        for (name, members) in &manifest.features {
            let members_str = members
                .iter()
                .map(|m| format!("\"{}\"", m))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("{} = [{}]\n", name, members_str));
        }
        if let Some(feat_extras) = manifest.section_extras.get("features") {
            for line in feat_extras {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (profile_name, profile) in &manifest.profiles {
        out.push_str(&format!("\n[profile.{}]\n", profile_name));
        out.push_str(&format!("opt-level = {}\n", profile.opt_level));
        out.push_str(&format!("debug = {}\n", profile.debug));
        out.push_str(&format!("lto = {}\n", profile.lto));
        if let Some(prof_extras) = manifest
            .section_extras
            .get(&format!("profile.{}", profile_name))
        {
            for line in prof_extras {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (section, lines) in &manifest.section_extras {
        if section.is_empty()
            || section == "package"
            || section == "dependencies"
            || section == "build"
            || section == "features"
            || section == "workspace"
            || section.starts_with("profile.")
        {
            continue;
        }
        out.push_str(&format!("\n[{}]\n", section));
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub fn parse_version_tuple(v: &str) -> Option<(u64, u64, u64)> {
    let clean = v.trim().trim_start_matches(|c| {
        c == 'v' || c == '^' || c == '~' || c == '=' || c == '>' || c == ' '
    });
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let major = parts[0].trim().parse::<u64>().ok()?;
    let minor = if parts.len() > 1 {
        parts[1].trim().parse::<u64>().ok()?
    } else {
        0
    };
    let patch = if parts.len() > 2 {
        let p = parts[2].split('-').next().unwrap_or(parts[2]).trim();
        p.parse::<u64>().ok()?
    } else {
        0
    };
    Some((major, minor, patch))
}

pub fn check_compiler_compatibility(manifest: &PackageManifest) -> Result<(), String> {
    if let Some(req_str) = &manifest.package.alya_version {
        let current_str = env!("CARGO_PKG_VERSION");
        if let (Some(req), Some(cur)) = (
            parse_version_tuple(req_str),
            parse_version_tuple(current_str),
        ) {
            if req > cur {
                return Err(format!(
                    "Package '{}' requires Alya compiler version >= {}, but current compiler version is {}.",
                    manifest.package.name, req_str, current_str
                ));
            }
        }
    }
    Ok(())
}

fn merge_multiline_toml(content: &str) -> Vec<(usize, String)> {
    let mut logical_lines = Vec::new();
    let mut current_buf = String::new();
    let mut start_line = 0;
    let mut bracket_depth = 0;
    let mut brace_depth = 0;

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        // Section header e.g. [package] should not be merged with subsequent lines
        if line.starts_with('[') && line.ends_with(']') && bracket_depth == 0 && brace_depth == 0 {
            if !current_buf.is_empty() {
                logical_lines.push((start_line, current_buf.trim().to_string()));
                current_buf.clear();
            }
            logical_lines.push((line_no + 1, line.to_string()));
            continue;
        }

        if current_buf.is_empty() {
            start_line = line_no + 1;
        } else {
            current_buf.push(' ');
        }
        current_buf.push_str(line);

        let mut in_str = false;
        let mut escaped = false;
        for ch in line.chars() {
            if ch == '\\' && in_str {
                escaped = !escaped;
                continue;
            }
            if ch == '"' && !escaped {
                in_str = !in_str;
            }
            if !in_str {
                match ch {
                    '[' => bracket_depth += 1,
                    ']' if bracket_depth > 0 => bracket_depth -= 1,
                    '{' => brace_depth += 1,
                    '}' if brace_depth > 0 => brace_depth -= 1,
                    _ => {}
                }
            }
            escaped = false;
        }

        if bracket_depth == 0 && brace_depth == 0 {
            logical_lines.push((start_line, current_buf.trim().to_string()));
            current_buf.clear();
        }
    }

    if !current_buf.is_empty() {
        logical_lines.push((start_line, current_buf.trim().to_string()));
    }

    logical_lines
}
