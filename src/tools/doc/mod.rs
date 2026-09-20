pub mod extractor;
pub mod html;
pub mod markdown;

use extractor::extract_module_docs;
use html::generate_html;
use markdown::generate_markdown;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_doc(
    input: &str,
    output_dir: Option<&str>,
    gen_html: bool,
    gen_markdown: bool,
) -> Result<(), String> {
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
    } else if input_path.is_dir() {
        process_directory(input_path, out_path, do_html, do_md)?;
    } else {
        return Err(format!("Invalid input path: {}", input));
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
) -> Result<(), String> {
    let mut alya_files = Vec::new();
    collect_alya_files(dir_path, &mut alya_files)?;

    if alya_files.is_empty() {
        println!("No .alya files found in {}", dir_path.display());
        return Ok(());
    }

    let mut modules = Vec::new();

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
            let html_content = generate_html(&module);
            let html_file = out_dir.join(format!("{}.html", target_file_name));
            fs::write(&html_file, html_content).map_err(|e| {
                format!("Failed to write HTML doc '{}': {}", html_file.display(), e)
            })?;
            println!("  ✓ Generated HTML doc: {}", html_file.display());
        }

        modules.push(module);
    }

    // Detect package name from alya.toml if available
    let mut pkg_name = None;
    let mut curr = dir_path.to_path_buf();
    loop {
        let manifest_path = curr.join("alya.toml");
        if manifest_path.exists() {
            if let Ok(manifest_src) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = crate::tools::pkg::manifest::parse_manifest(&manifest_src) {
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
        let index_html = crate::tools::doc::html::generate_index_html(&modules);
        let index_file = out_dir.join("index.html");
        let _ = fs::write(&index_file, index_html);
        println!("  ✓ Generated HTML Index: {}", index_file.display());
    }

    Ok(())
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
