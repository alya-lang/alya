pub mod extractor;
pub mod html;
pub mod markdown;

use extractor::{extract_module_docs, DocModule};
use html::{generate_html, generate_html_with_nav};
use markdown::generate_markdown;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_doc(
    input: &str,
    output_dir: Option<&str>,
    gen_html: bool,
    gen_markdown: bool,
    up_link: Option<(String, String)>,
) -> Result<Vec<extractor::DocModule>, String> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        return Err(format!("Input path does not exist: {}", input));
    }

    let out_dir = output_dir.unwrap_or("docs");
    let out_path = Path::new(out_dir);
    fs::create_dir_all(out_path)
        .map_err(|e| format!("Failed to create output directory '{}': {}", out_dir, e))?;

    let (do_html, do_md) = match (gen_html, gen_markdown) {
        (false, false) => (true, true), // default to both
        (h, m) => (h, m),
    };

    if input_path.is_file() {
        process_single_file(input_path, out_path, do_html, do_md)?;
        Ok(Vec::new())
    } else if input_path.is_dir() {
        process_directory(input_path, out_path, do_html, do_md, up_link)
    } else {
        Err(format!("Invalid input path: {}", input))
    }
}

/// Documents every workspace member into `<out>/<member>/` plus a root
/// index linking the members. `out` defaults to `docs` under the cwd.
/// Member directories arrive absolute, so no root is needed.
pub fn run_doc_workspace(
    members: &[crate::tools::pkg::workspace::WorkspaceMember],
    output_dir: Option<&str>,
    gen_html: bool,
    gen_markdown: bool,
) -> Result<(), String> {
    if members.is_empty() {
        return Err("Error: workspace resolves to no members".to_string());
    }
    let (do_html, do_md) = match (gen_html, gen_markdown) {
        (false, false) => (true, true), // default to both
        (h, m) => (h, m),
    };
    let out_root = output_dir.unwrap_or("docs");
    let out_path = Path::new(out_root);
    fs::create_dir_all(out_path)
        .map_err(|e| format!("Failed to create output directory '{}': {}", out_root, e))?;
    // Per-member extracted modules: merged into hub cards below so the
    // root index badges carry real symbol totals.
    let mut hub_modules: Vec<Vec<extractor::DocModule>> = Vec::new();

    for member in members {
        println!("\n--- workspace member: {} ---", member.name);
        // Member docs read like a standalone package: `src/` when present,
        // else the member dir (mirrors the `alya doc` default).
        let src_dir = member.dir.join("src");
        let input = if src_dir.is_dir() {
            src_dir.to_string_lossy().replace('\\', "/")
        } else {
            member.dir.to_string_lossy().replace('\\', "/")
        };
        let member_out = out_path.join(&member.name);
        let member_modules = run_doc(
            &input,
            Some(&member_out.to_string_lossy().replace('\\', "/")),
            do_html,
            do_md,
            Some(("Workspace".to_string(), "../index.html".to_string())),
        )?;
        hub_modules.push(member_modules);
        // Markdown trees have no sidebar: every member page gets a footer
        // link back to the workspace root index. Idempotent across re-runs.
        if do_md {
            append_workspace_md_footer(&member_out)?;
        }
    }

    // Root index linking the per-member trees.
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
    if do_md {
        let mut index_md = String::from("# Workspace API Documentation\n\n");
        index_md.push_str(&format!(
            "API reference index for {} workspace member{}.\n\n",
            names.len(),
            if names.len() == 1 { "" } else { "s" }
        ));
        index_md.push_str("| Member | API Link |\n| :--- | :--- |\n");
        for name in &names {
            index_md.push_str(&format!(
                "| `{}` | [{}/index.md]({}/index.md) |\n",
                name, name, name
            ));
        }
        let index_file = out_path.join("index.md");
        fs::write(&index_file, index_md)
            .map_err(|e| format!("Failed to write index '{}': {}", index_file.display(), e))?;
        println!("  ✓ Generated Index: {}", index_file.display());
    }
    if do_html {
        // One synthetic module per member so the root index renders with
        // the standard design (same chrome, sidebar, and search as member
        // pages). Member item vectors merge in, so badges carry real symbol
        // totals instead of zeros. Generated links (`<member>.html`) are
        // rewritten to the per-member trees (`<member>/index.html`).
        let mut hub: Vec<extractor::DocModule> = Vec::new();
        for (member, mods) in members.iter().zip(hub_modules.iter()) {
            let mut module = extractor::DocModule::new(&member.name, &member.name);
            let manifest_path = member.dir.join("alya.toml");
            if let Ok(src) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = crate::tools::pkg::manifest::parse_manifest(&src) {
                    module.description = manifest.package.description.unwrap_or_default();
                }
            }
            for m in mods {
                module.functions.extend(m.functions.iter().cloned());
                module.structs.extend(m.structs.iter().cloned());
                module.enums.extend(m.enums.iter().cloned());
                module.interfaces.extend(m.interfaces.iter().cloned());
                module.constants.extend(m.constants.iter().cloned());
            }
            hub.push(module);
        }
        let mut index_html =
            crate::tools::doc::html::generate_index_html(&hub, Some("workspace"), None);
        for member in members {
            let from = format!("\"{}.html\"", member.name);
            let to = format!("\"{}/index.html\"", member.name);
            index_html = index_html.replace(&from, &to);
        }
        let index_file = out_path.join("index.html");
        fs::write(&index_file, index_html)
            .map_err(|e| format!("Failed to write index '{}': {}", index_file.display(), e))?;
        println!("  ✓ Generated HTML Index: {}", index_file.display());
    }
    Ok(())
}

/// Appends navigation footers to every markdown page of one member
/// tree. Module pages link to the member index (`All modules`) and the
/// workspace root (`Workspace`); the member index itself links only to the
/// workspace root (an All-modules link there would be self-referential —
/// same single-link rule as the HTML footer).
fn append_workspace_md_footer(member_out: &Path) -> Result<(), String> {
    const WS_FOOTER: &str = "\n---\n\n[\u{2191} Workspace](../index.md)\n";
    const MOD_FOOTER: &str =
        "\n---\n\n[\u{2190} All modules](index.md) \u{00b7} [\u{2191} Workspace](../index.md)\n";
    let entries = fs::read_dir(member_out)
        .map_err(|e| format!("Failed to read '{}': {}", member_out.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_md = path.is_file() && path.extension().is_some_and(|e| e == "md");
        if !is_md {
            continue;
        }
        let is_index = path.file_name().is_some_and(|n| n == "index.md");
        let footer = if is_index { WS_FOOTER } else { MOD_FOOTER };
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        if content.ends_with(WS_FOOTER) || content.ends_with(MOD_FOOTER) {
            continue;
        }
        fs::write(&path, format!("{}{}", content.trim_end(), footer))
            .map_err(|e| format!("Failed to write '{}': {}", path.display(), e))?;
    }
    Ok(())
}

fn process_single_file(
    file_path: &Path,
    out_dir: &Path,
    gen_html: bool,
    gen_markdown: bool,
) -> Result<(), String> {
    let source = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path.display(), e))?;

    let module = extract_module_docs(&source, &file_path.to_string_lossy());
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc");

    if gen_markdown {
        let md_content = generate_markdown(&module);
        let md_file = out_dir.join(format!("{}.md", stem));
        fs::write(&md_file, md_content)
            .map_err(|e| format!("Failed to write markdown '{}': {}", md_file.display(), e))?;
        println!("  ✓ Generated Markdown doc: {}", md_file.display());
    }

    if gen_html {
        let html_content = generate_html(&module);
        let html_file = out_dir.join(format!("{}.html", stem));
        fs::write(&html_file, html_content)
            .map_err(|e| format!("Failed to write HTML doc '{}': {}", html_file.display(), e))?;
        println!("  ✓ Generated HTML doc: {}", html_file.display());
    }

    Ok(())
}

fn process_directory(
    dir_path: &Path,
    out_dir: &Path,
    gen_html: bool,
    gen_markdown: bool,
    up_link: Option<(String, String)>,
) -> Result<Vec<extractor::DocModule>, String> {
    let mut alya_files = Vec::new();
    collect_alya_files(dir_path, &mut alya_files)?;

    if alya_files.is_empty() {
        println!("No .alya files found in {}", dir_path.display());
        return Ok(Vec::new());
    }

    let mut modules = Vec::new();
    // HTML pages render after all modules are known so each page can link
    // its sibling modules in the sidebar.
    let mut html_jobs: Vec<(DocModule, String)> = Vec::new();

    for file in &alya_files {
        let source = fs::read_to_string(file)
            .map_err(|e| format!("Failed to read file '{}': {}", file.display(), e))?;

        let rel_path = file.strip_prefix(dir_path).unwrap_or(file);
        let mod_name = rel_path
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        let mut module = extract_module_docs(&source, &file.to_string_lossy());
        module.name = mod_name;

        let target_file_name = module.name.replace('/', "_");

        if gen_markdown {
            let md_content = generate_markdown(&module);
            let md_file = out_dir.join(format!("{}.md", target_file_name));
            fs::write(&md_file, md_content)
                .map_err(|e| format!("Failed to write markdown '{}': {}", md_file.display(), e))?;
            println!("  ✓ Generated Markdown doc: {}", md_file.display());
        }

        if gen_html {
            let target_file = out_dir.join(format!("{}.html", target_file_name));
            html_jobs.push((module.clone(), target_file.to_string_lossy().to_string()));
        }

        modules.push(module);
    }

    // HTML rendering runs after collection so sibling navigation is complete.
    if gen_html {
        for (module, target_file) in &html_jobs {
            let html_content = generate_html_with_nav(
                module,
                &modules,
                up_link
                    .as_ref()
                    .map(|(label, href)| (label.as_str(), href.as_str())),
            );
            fs::write(target_file, html_content)
                .map_err(|e| format!("Failed to write HTML doc '{}': {}", target_file, e))?;
            println!("  ✓ Generated HTML doc: {}", target_file);
        }
    }

    // Detect package name from alya.toml if available. Virtual workspace
    // roots (members only) are skipped: they have no package payload, and
    // their synthesized name must never leak into documentation titles.
    let mut pkg_name = None;
    let mut curr = dir_path.to_path_buf();
    loop {
        let manifest_path = curr.join("alya.toml");
        if manifest_path.exists() {
            if let Ok(manifest_src) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = crate::tools::pkg::manifest::parse_manifest(&manifest_src) {
                    if crate::tools::pkg::manifest::is_virtual_workspace_root(&manifest) {
                        if !curr.pop() {
                            break;
                        }
                        continue;
                    }
                    pkg_name = Some(manifest.package.name);
                    break;
                }
            }
        }
        if !curr.pop() {
            break;
        }
    }

    // Generate index.html and index.md
    if gen_markdown {
        let mut index_md = String::new();
        if let Some(ref name) = pkg_name {
            index_md.push_str(&format!("# {} API Documentation\n\n", name));
            index_md.push_str(&format!("Official API reference index for `{}`.\n\n", name));
        } else {
            index_md.push_str("# Alya Standard Library Documentation\n\n");
            index_md
                .push_str("Official API reference index across all standard library modules.\n\n");
        }
        index_md.push_str("| Module | Description | API Link |\n");
        index_md.push_str("| :--- | :--- | :--- |\n");
        for m in &modules {
            let target_file_name = m.name.replace('/', "_");
            let desc = if !m.description.is_empty() {
                m.description
                    .lines()
                    .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                    .unwrap_or(&m.description)
                    .to_string()
            } else {
                "-".to_string()
            };
            let mod_display = if pkg_name.is_some() {
                m.name.clone()
            } else {
                format!("std/{}", m.name)
            };
            index_md.push_str(&format!(
                "| `{}` | {} | [{}.md]({}.md) |\n",
                mod_display, desc, m.name, target_file_name
            ));
        }
        let index_file = out_dir.join("index.md");
        let _ = fs::write(&index_file, index_md);
        println!("  ✓ Generated Index: {}", index_file.display());
    }

    if gen_html {
        let index_html = crate::tools::doc::html::generate_index_html(
            &modules,
            pkg_name.as_deref(),
            up_link
                .as_ref()
                .map(|(label, href)| (label.as_str(), href.as_str())),
        );
        let index_file = out_dir.join("index.html");
        let _ = fs::write(&index_file, index_html);
        println!("  ✓ Generated HTML Index: {}", index_file.display());
    }

    Ok(modules)
}

fn collect_alya_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str != ".git"
                    && name_str != ".alya"
                    && name_str != "target"
                    && name_str != "docs"
                    && name_str != "tests"
                    && name_str != "benches"
                    && name_str != "examples"
                    && name_str != "node_modules"
                {
                    collect_alya_files(&path, out)?;
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "alya" {
                        out.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}
