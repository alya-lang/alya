use super::toml::{parse_inline_table, parse_string_array, strip_toml_comment, unquote};
use super::types::{BuildConfig, DependencySource, PackageInfo, PackageManifest};
use std::collections::BTreeMap;

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
        let known = matches!(current.as_str(), "" | "package" | "dependencies" | "build");
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
    let mut c_flags_windows = Vec::new();
    let mut c_flags_macos = Vec::new();
    let mut c_flags_linux = Vec::new();
    let mut build_extra = BTreeMap::new();
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
                    "c-flags-windows" | "c_flags_windows" => {
                        c_flags_windows = parse_string_array(val)
                    }
                    "c-flags-macos" | "c_flags_macos" => c_flags_macos = parse_string_array(val),
                    "c-flags-linux" | "c_flags_linux" => c_flags_linux = parse_string_array(val),
                    "c-flags" | "c_flags" => c_flags = parse_string_array(val),
                    "c-include-dirs" | "c_include_dirs" => c_include_dirs = parse_string_array(val),
                    other => {
                        build_extra.insert(other.to_string(), val.to_string());
                    }
                },
                "dependencies" => {
                    if val.starts_with('{') {
                        let table = parse_inline_table(val);
                        if let Some(p) = table.get("path") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Path { path: p.clone() },
                            );
                        } else if let Some(g) = table.get("git") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Git {
                                    url: g.clone(),
                                    tag: table.get("tag").cloned(),
                                    branch: table.get("branch").cloned(),
                                    rev: table.get("rev").cloned(),
                                },
                            );
                        } else if let Some(v_inner) = table.get("version") {
                            dependencies.insert(
                                key.to_string(),
                                DependencySource::Version(v_inner.clone()),
                            );
                        } else {
                            return Err(format!(
                                "Invalid dependency entry '{}' in alya.toml at line {}: inline table must declare one of 'path', 'git', or 'version'",
                                key, line_no
                            ));
                        }
                    } else {
                        dependencies
                            .insert(key.to_string(), DependencySource::Version(unquote(val)));
                    }
                }
                _ => {}
            }
        } else {
            return Err(format!(
                "Syntax error in alya.toml at line {}: '{}'",
                line_no, line
            ));
        }
    }

    if name.is_empty() {
        return Err("Missing required field 'name' under [package] in alya.toml".to_string());
    }

    let build = if !c_sources.is_empty()
        || !c_flags.is_empty()
        || !c_include_dirs.is_empty()
        || !c_sources_windows.is_empty()
        || !c_sources_macos.is_empty()
        || !c_sources_linux.is_empty()
        || !c_flags_windows.is_empty()
        || !c_flags_macos.is_empty()
        || !c_flags_linux.is_empty()
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
            c_flags_windows,
            c_flags_macos,
            c_flags_linux,
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
        section_extras,
    })
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

    out.push_str("\n[dependencies]\n");
    for (name, dep) in &manifest.dependencies {
        match dep {
            DependencySource::Version(v) => {
                out.push_str(&format!("{} = \"{}\"\n", name, v));
            }
            DependencySource::Path { path } => {
                out.push_str(&format!(
                    "{} = {{ path = \"{}\" }}\n",
                    name,
                    path.replace('\\', "/")
                ));
            }
            DependencySource::Git {
                url,
                tag,
                branch,
                rev,
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
            ("c-flags-windows", &b.c_flags_windows),
            ("c-flags-macos", &b.c_flags_macos),
            ("c-flags-linux", &b.c_flags_linux),
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
    for (section, lines) in &manifest.section_extras {
        if section.is_empty()
            || section == "package"
            || section == "dependencies"
            || section == "build"
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
